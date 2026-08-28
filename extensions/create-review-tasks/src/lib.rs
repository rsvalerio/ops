//! Generic engine for `ops create-review-tasks`.
//!
//! Mirrors the about split: this crate owns the stack-independent work —
//! querying the `review_targets` data provider, allocating backlog ids, and
//! writing backlog.md task files — while stack-specific crates
//! (`extensions-<stack>/create-review-tasks`) register the provider that
//! decides what a review target is for their stack.
//!
//! Task shapes (main `review-request-<date>-<n>` plus one `REVIEW: Run skill
//! <skill> against <target>` subtask per review target) are rendered by
//! [`backlog`] to be byte-compatible with the backlog.md CLI.

mod backlog;
mod clock;

use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use anyhow::Context as _;
use ops_core::config::Config;
use ops_extension::{Context, DataProviderError, DataRegistry};
use serde::Deserialize;

use clock::UtcStamp;

/// Name of the data provider this engine queries. Stack-specific
/// create-review-tasks extensions register under this name.
pub const DATA_PROVIDER_NAME: &str = "review_targets";

/// Whether a run writes task files or only reports what it would create.
///
/// PATTERN-1: the mode is a type, not a bool, so call sites read
/// `RunMode::DryRun` instead of decoding a bare `true`. Both modes share
/// every step except the file writes — a dry run validates the backlog tree,
/// queries the provider, and allocates ids exactly like a real run, so a
/// succeeding dry run is a faithful predictor of the run that follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunMode {
    /// Write the task files under `.backlog/tasks/`.
    Write,
    /// Report the tasks that would be created, touching nothing on disk.
    DryRun,
}

/// Payload contract of the [`DATA_PROVIDER_NAME`] provider: the review skill
/// to invoke (e.g. `code-review-rust`) plus one target per review unit.
#[derive(Debug, Clone, Deserialize)]
pub struct ReviewTargets {
    /// Skill name the subtask titles reference, e.g. `code-review-rust`.
    pub skill: String,
    /// Review targets, one subtask each.
    pub targets: Vec<ReviewTarget>,
}

/// One review target: a display name (unique per workspace, e.g. the cargo
/// package name) and its member path relative to the workspace root.
#[derive(Debug, Clone, Deserialize)]
pub struct ReviewTarget {
    /// Display name used in the subtask title.
    pub name: String,
    /// Member path relative to the workspace root (summary context only).
    pub path: String,
}

/// Bound on allocation retries when another writer wins the race to a task
/// filename. Every retry rescans the backlog tree, so the number it picks
/// strictly advances past the winner; the bound only guards against a
/// pathological writer creating tasks faster than this run can allocate them.
const MAX_ALLOCATION_ATTEMPTS: u32 = 32;

/// Create the review-request task set: one main task plus one subtask per
/// review target, written as markdown files under `.backlog/tasks/`.
///
/// [`RunMode::DryRun`] prints the same report with `would create` verbs and
/// writes nothing.
///
/// The set is all-or-nothing: every file is created exclusively, a filename
/// another run already took sends this one back to reallocate its ids, and
/// any failure removes the files this run staged before returning. The
/// report is printed only once the whole set is on disk.
///
/// # Errors
///
/// - No `.backlog/tasks` directory in `workspace_root` (hint: `backlog init`).
/// - No `review_targets` provider registered for the detected stack.
/// - The registered provider failed, or its payload did not match
///   [`ReviewTargets`].
/// - The host clock reads before 1970-01-01, so the run has no date to stamp
///   the task set with.
/// - A provider-supplied `skill`, target name or target path is empty,
///   over-long, or carries a control character.
/// - A task file could not be written (the error names the path;
///   [`RunMode::DryRun`] never takes this branch).
/// - [`MAX_ALLOCATION_ATTEMPTS`] consecutive allocations all lost the race to
///   a concurrent writer ([`RunMode::DryRun`] never takes this branch).
pub fn run_create_review_tasks(
    registry: &DataRegistry,
    workspace_root: &Path,
    out: &mut dyn Write,
    mode: RunMode,
) -> anyhow::Result<()> {
    run_create_review_tasks_with_clock(registry, workspace_root, out, mode, UtcStamp::now)
}

/// [`run_create_review_tasks`] against an explicit clock.
///
/// ERR-6: the clock is read first and its failure aborts the run, in both
/// modes, before any id is allocated or any file created — an unreadable
/// clock must never date a whole task set `1970-01-01`.
fn run_create_review_tasks_with_clock(
    registry: &DataRegistry,
    workspace_root: &Path,
    out: &mut dyn Write,
    mode: RunMode,
    clock: impl FnOnce() -> anyhow::Result<UtcStamp>,
) -> anyhow::Result<()> {
    let stamp = clock()?;
    run_create_review_tasks_at(registry, workspace_root, out, mode, &stamp)
}

/// Clock-injecting core of [`run_create_review_tasks`]; the `stamp` pins the
/// `created_date` frontmatter and the main-task title date for tests.
fn run_create_review_tasks_at(
    registry: &DataRegistry,
    workspace_root: &Path,
    out: &mut dyn Write,
    mode: RunMode,
    stamp: &UtcStamp,
) -> anyhow::Result<()> {
    backlog::require_backlog_tasks_dir(workspace_root)?;
    let targets = fetch_review_targets(registry, workspace_root)?;
    if targets.targets.is_empty() {
        anyhow::bail!(
            "review_targets provider returned no targets — nothing to create review tasks for"
        );
    }

    let plan = match mode {
        // A dry run allocates exactly like a real run and stops there, so the
        // ids it reports are the ones the next real run will try to take.
        RunMode::DryRun => plan_task_set(workspace_root, &targets, stamp),
        RunMode::Write => commit_task_set(workspace_root, &targets, stamp)?,
    };
    // Deferred until the set exists: the operator is never told a task was
    // created that a rollback then removed.
    report(out, &plan, mode)
}

/// Query the [`DATA_PROVIDER_NAME`] provider and decode its payload.
fn fetch_review_targets(
    registry: &DataRegistry,
    workspace_root: &Path,
) -> anyhow::Result<ReviewTargets> {
    // The provider only consumes `ctx.working_directory`; matching the
    // ops-about precedent, an empty Config is sufficient here.
    let config = Arc::new(Config::empty());
    let mut ctx = Context::new(config, workspace_root.to_path_buf());
    let payload = registry
        .provide(DATA_PROVIDER_NAME, &mut ctx)
        .map_err(|err| match err {
            DataProviderError::NotFound(name) => anyhow::anyhow!(
                "no {name} provider registered — the detected stack has no \
                 create-review-tasks extension compiled in"
            ),
            other => anyhow::Error::new(other),
        })
        .with_context(|| format!("collecting {DATA_PROVIDER_NAME}"))?;
    let targets: ReviewTargets = serde_json::from_value(payload)
        .with_context(|| format!("decoding {DATA_PROVIDER_NAME} payload"))?;
    validate(&targets).with_context(|| format!("validating {DATA_PROVIDER_NAME} payload"))?;
    Ok(targets)
}

/// Longest accepted `skill` or target `name`, in characters. Both end up in a
/// task title, a YAML scalar and a filename slug; a workspace member name far
/// past this is a mistake or an attack, not a display name.
const MAX_NAME_CHARS: usize = 200;
/// Longest accepted target `path`, in characters — generous enough for any
/// real workspace-relative member path.
const MAX_PATH_CHARS: usize = 1_024;

/// SEC-11 layer 3 (format): validate the decoded payload at the one boundary
/// it enters through, rather than hardening each of the three sinks it
/// reaches — the YAML frontmatter, the on-disk filename, and the stdout
/// report. A repository under review controls these strings (the Rust
/// provider reads them from each member's `[package].name`), so a newline
/// would otherwise break the frontmatter out of its scalar and an ANSI escape
/// would rewrite the operator's terminal around the run report.
fn validate(targets: &ReviewTargets) -> anyhow::Result<()> {
    validate_field("skill", &targets.skill, MAX_NAME_CHARS)?;
    for (position, target) in targets.targets.iter().enumerate() {
        // `position` indexes an in-memory slice, so `saturating_add` is exact.
        let index = position.saturating_add(1);
        validate_field(
            &format!("targets[{index}].name"),
            &target.name,
            MAX_NAME_CHARS,
        )?;
        validate_field(
            &format!("targets[{index}].path"),
            &target.path,
            MAX_PATH_CHARS,
        )?;
    }
    Ok(())
}

/// Reject one payload string that cannot be carried safely to the sinks
/// above. Offending values are rendered with `{:?}`, which escapes control
/// characters, so the rejection itself cannot forge terminal output.
fn validate_field(field: &str, value: &str, max_chars: usize) -> anyhow::Result<()> {
    if value.is_empty() {
        anyhow::bail!("{field} is empty");
    }
    let chars = value.chars().count();
    if chars > max_chars {
        anyhow::bail!("{field} is {chars} characters long, over the {max_chars}-character bound");
    }
    if let Some(control) = value.chars().find(|ch| ch.is_control()) {
        anyhow::bail!(
            "{field} contains the control character {control:?}, which cannot appear in a task \
             title, filename or report line: {value:?}"
        );
    }
    Ok(())
}

/// The task set one run allocates: the main task plus one subtask per review
/// target, with every id fixed by a single scan of the backlog tree.
struct TaskPlan<'a> {
    /// Main-task number; the id and both filename families share it.
    main_number: u32,
    /// Zero-padded main-task id, e.g. `TASK-0042`.
    main_id: String,
    /// Main-task title, `review-request-<date>-<n>`.
    main_title: String,
    /// Subtasks, index-aligned with the provider's targets.
    subtasks: Vec<PlannedSubtask<'a>>,
}

/// One planned subtask. FN-3: the identity fields travel as one struct rather
/// than as positional parameters.
struct PlannedSubtask<'a> {
    /// 1-based position among the sibling subtasks.
    index: usize,
    /// Dotted id, e.g. `TASK-0042.03`.
    id: String,
    /// Subtask title.
    title: String,
    /// Review target path; report context only, never part of a filename.
    path: &'a str,
}

/// Allocate the ids for one attempt. Pure apart from the single directory
/// scan, so a dry run and a real run reach identical plans from identical
/// state.
fn plan_task_set<'a>(
    workspace_root: &Path,
    targets: &'a ReviewTargets,
    stamp: &UtcStamp,
) -> TaskPlan<'a> {
    let backlog::NextIds {
        number: main_number,
        sequence,
    } = backlog::next_ids(workspace_root, &stamp.date);
    let subtasks = targets
        .targets
        .iter()
        .enumerate()
        .map(|(position, target)| {
            // `position` indexes an in-memory slice, so it is at most
            // `len - 1` and `saturating_add(1)` is exact.
            let index = position.saturating_add(1);
            PlannedSubtask {
                index,
                id: backlog::subtask_id(main_number, index),
                title: format!(
                    "REVIEW: Run skill {} against {}",
                    targets.skill, target.name
                ),
                path: &target.path,
            }
        })
        .collect();
    TaskPlan {
        main_number,
        main_id: backlog::main_task_id(main_number),
        main_title: format!("review-request-{}-{sequence}", stamp.date),
        subtasks,
    }
}

/// Allocate and write the whole task set, returning the plan that committed.
///
/// SEC-25: both ids come from one directory scan, so they are consistent with
/// each other by construction, but that scan is still not atomic with the
/// write that follows it — the remaining race is purely time-of-check to
/// time-of-use. Two guards close it. Every file is created
/// with `create_new`, which makes the filesystem — not the scan — decide who
/// owns a filename; and once the main task file exists,
/// [`backlog::conflicting_claim`] re-reads the tree to catch a concurrent run
/// that took the same main number or the same daily sequence under a
/// *different* name. A loser on either guard rolls back and rescans instead of
/// leaving a duplicate id or a duplicate `review-request-<date>-<n>` title
/// behind.
fn commit_task_set<'a>(
    workspace_root: &Path,
    targets: &'a ReviewTargets,
    stamp: &UtcStamp,
) -> anyhow::Result<TaskPlan<'a>> {
    for _ in 0..MAX_ALLOCATION_ATTEMPTS {
        let plan = plan_task_set(workspace_root, targets, stamp);
        if write_task_set(workspace_root, &plan, stamp)? {
            return Ok(plan);
        }
    }
    anyhow::bail!(
        "gave up allocating a free review-request id after {MAX_ALLOCATION_ATTEMPTS} attempts — \
         another process is creating tasks in {} concurrently",
        workspace_root.join(".backlog").join("tasks").display()
    )
}

/// Write one attempt's task set. `Ok(true)` means every file committed;
/// `Ok(false)` means another run already holds one of this attempt's
/// identifiers — its filename, its main number, or its daily sequence — and
/// the caller must reallocate. Either way — and on any error — the files this attempt created
/// are gone by the time it returns.
fn write_task_set(
    workspace_root: &Path,
    plan: &TaskPlan<'_>,
    stamp: &UtcStamp,
) -> anyhow::Result<bool> {
    let tasks_dir = workspace_root.join(".backlog").join("tasks");
    let mut staged = StagedTasks::new();

    let main = TaskFile {
        name: backlog::main_task_file_name(plan.main_number, &plan.main_title),
        id: &plan.main_id,
        title: &plan.main_title,
        subtask_of: None,
    };
    if !stage_task_file(&mut staged, &tasks_dir, &main, stamp)? {
        return Ok(false);
    }
    // The main file now exists, so it is a reservation every concurrent run
    // can see. Re-reading the tree here catches the allocations that `create_new`
    // structurally cannot: a run that interleaved between this one's two id
    // scans and landed on the same number or the same daily sequence under a
    // different filename. Seeing a conflict always loses, so two runs never
    // both keep — whichever checks last sees both files.
    let claim = backlog::MainTaskClaim {
        file_name: &main.name,
        number: plan.main_number,
        title: &plan.main_title,
    };
    if let Some(other) = backlog::conflicting_claim(workspace_root, &claim) {
        tracing::debug!(
            conflict = %other,
            claimed = %plan.main_title,
            "review-request id claimed concurrently; reallocating"
        );
        return Ok(false);
    }
    for subtask in &plan.subtasks {
        let file = TaskFile {
            name: backlog::subtask_file_name(plan.main_number, subtask.index, &subtask.title),
            id: &subtask.id,
            title: &subtask.title,
            subtask_of: Some((&plan.main_id, subtask.index)),
        };
        if !stage_task_file(&mut staged, &tasks_dir, &file, stamp)? {
            return Ok(false);
        }
    }
    staged.keep();
    Ok(true)
}

/// One task file to create. FN-3: grouped rather than passed as five
/// positional parameters.
struct TaskFile<'a> {
    /// Filename inside `.backlog/tasks/`.
    name: String,
    /// Zero-padded id, e.g. `TASK-0042` or `TASK-0042.03`.
    id: &'a str,
    /// Task title.
    title: &'a str,
    /// Parent id plus 1-based position for a subtask; `None` for a main task.
    subtask_of: Option<(&'a str, usize)>,
}

/// Create and render one task file, registering it with `staged` first so a
/// failure part-way through the render still rolls it back. `Ok(false)` means
/// the name was taken between the id scan and the create.
///
/// ERR-13: both the create and the write errors name the exact file path.
fn stage_task_file(
    staged: &mut StagedTasks,
    tasks_dir: &Path,
    file: &TaskFile<'_>,
    stamp: &UtcStamp,
) -> anyhow::Result<bool> {
    let path = tasks_dir.join(&file.name);
    // SEC-25: `create_new` is the atomic check-and-create. `File::create`
    // would silently truncate a task file another run just wrote.
    let mut handle = match std::fs::File::create_new(&path) {
        Ok(handle) => handle,
        Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => return Ok(false),
        Err(err) => {
            return Err(anyhow::Error::new(err))
                .with_context(|| format!("creating {}", path.display()))
        }
    };
    staged.track(path.clone());
    backlog::render_task_file(&mut handle, file.id, file.title, stamp, file.subtask_of)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(true)
}

/// The task files one attempt has created. Dropping this without
/// [`StagedTasks::keep`] deletes them, so neither a lost allocation race nor
/// a failed write leaves a half-created set behind — and a half-created set
/// is worse than none, because the backlog CLI would show a review request
/// whose subtasks silently stop short of the targets it names.
struct StagedTasks {
    /// Paths created so far, in creation order.
    paths: Vec<std::path::PathBuf>,
    /// Set once the whole set is on disk; suppresses the rollback.
    committed: bool,
}

impl StagedTasks {
    /// An attempt with nothing staged yet.
    const fn new() -> Self {
        Self {
            paths: Vec::new(),
            committed: false,
        }
    }

    /// Register a file this attempt created.
    fn track(&mut self, path: std::path::PathBuf) {
        self.paths.push(path);
    }

    /// Keep every staged file: the set is complete.
    const fn keep(&mut self) {
        self.committed = true;
    }
}

impl Drop for StagedTasks {
    fn drop(&mut self) {
        if self.committed {
            return;
        }
        for path in self.paths.iter().rev() {
            // Best effort: the caller is already returning an error or about
            // to retry, and a failed cleanup must not mask that outcome.
            let _ = std::fs::remove_file(path);
        }
    }
}

/// Print the run report for a committed (or, in a dry run, planned) set.
///
/// SEC-11: every provider string reaching this sink passed [`validate`], so
/// no title or path can carry a newline or an ANSI escape that would forge or
/// rewrite report lines.
fn report(out: &mut dyn Write, plan: &TaskPlan<'_>, mode: RunMode) -> anyhow::Result<()> {
    let verb = match mode {
        RunMode::Write => "created",
        RunMode::DryRun => "would create",
    };
    writeln!(
        out,
        "{verb} {} {} ({} subtasks)",
        plan.main_id,
        plan.main_title,
        plan.subtasks.len()
    )?;
    for subtask in &plan.subtasks {
        writeln!(
            out,
            "{verb} {} {} ({})",
            subtask.id, subtask.title, subtask.path
        )?;
    }
    // Printed in both modes: even a dry run shows the id the eventual real
    // run will allocate, so the filter is usable as soon as the tasks exist.
    writeln!(
        out,
        "list subtasks: backlog task list --parent {}",
        plan.main_id
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_extension::DataProvider;

    /// Minimal provider returning a fixed JSON payload; enough to drive the
    /// engine without a stack-specific crate.
    struct FixedTargets(serde_json::Value);

    impl DataProvider for FixedTargets {
        fn name(&self) -> &'static str {
            DATA_PROVIDER_NAME
        }

        fn provide(&self, _ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
            Ok(self.0.clone())
        }
    }

    fn registry_with(payload: serde_json::Value) -> DataRegistry {
        let mut registry = DataRegistry::new();
        registry.register(DATA_PROVIDER_NAME, Box::new(FixedTargets(payload)));
        registry
    }

    fn sample_payload() -> serde_json::Value {
        serde_json::json!({
            "skill": "code-review-rust",
            "targets": [
                { "name": "ops-core", "path": "crates/core" },
                { "name": "ops-cli", "path": "crates/cli" }
            ]
        })
    }

    fn fixed_stamp() -> UtcStamp {
        UtcStamp {
            date: "2026-08-20".to_string(),
            minutes: "19:02".to_string(),
        }
    }

    fn scratch_backlog() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".backlog").join("tasks")).expect("tasks dir");
        dir
    }

    fn run(
        dir: &tempfile::TempDir,
        registry: &DataRegistry,
        mode: RunMode,
    ) -> (String, anyhow::Result<()>) {
        let mut out = Vec::new();
        let result =
            run_create_review_tasks_at(registry, dir.path(), &mut out, mode, &fixed_stamp());
        (String::from_utf8(out).expect("utf8"), result)
    }

    #[test]
    fn writes_main_task_and_one_subtask_per_target() {
        let dir = scratch_backlog();
        let registry = registry_with(sample_payload());
        let (out, result) = run(&dir, &registry, RunMode::Write);
        result.expect("run must succeed");

        let tasks_dir = dir.path().join(".backlog").join("tasks");
        let main_path = tasks_dir.join("task-0001 - review-request-2026-08-20-1.md");
        let main = std::fs::read_to_string(&main_path).expect("main task file");
        assert!(main.contains("id: TASK-0001\n"), "got: {main}");
        assert!(main.contains("title: 'review-request-2026-08-20-1'\n"));
        assert!(main.contains("ordinal: 1000\n"));
        assert!(!main.contains("parent_task_id"));

        let sub1 = std::fs::read_to_string(
            tasks_dir.join("task-0001.01 - REVIEW-Run-skill-code-review-rust-against-ops-core.md"),
        )
        .expect("subtask 1 file");
        assert!(sub1.contains("id: TASK-0001.01\n"), "got: {sub1}");
        assert!(sub1.contains("parent_task_id: TASK-0001\n"));
        assert!(sub1.contains("ordinal: 2001\n"));

        let sub2 = std::fs::read_to_string(
            tasks_dir.join("task-0001.02 - REVIEW-Run-skill-code-review-rust-against-ops-cli.md"),
        )
        .expect("subtask 2 file");
        assert!(sub2.contains("id: TASK-0001.02\n"));
        assert!(sub2.contains("ordinal: 2002\n"));

        assert_eq!(
            out,
            concat!(
                "created TASK-0001 review-request-2026-08-20-1 (2 subtasks)\n",
                "created TASK-0001.01 REVIEW: Run skill code-review-rust against ops-core (crates/core)\n",
                "created TASK-0001.02 REVIEW: Run skill code-review-rust against ops-cli (crates/cli)\n",
                "list subtasks: backlog task list --parent TASK-0001\n",
            )
        );
    }

    /// Second run on the same day must allocate the next main number and the
    /// next per-day sequence, never overwriting the first set.
    #[test]
    fn second_run_same_day_increments_number_and_sequence() {
        let dir = scratch_backlog();
        let registry = registry_with(sample_payload());
        let (_out1, result1) = run(&dir, &registry, RunMode::Write);
        result1.expect("first run");
        let (out2, result2) = run(&dir, &registry, RunMode::Write);
        result2.expect("second run");

        assert!(
            out2.starts_with("created TASK-0002 review-request-2026-08-20-2 (2 subtasks)\n"),
            "got: {out2}"
        );
        assert!(dir
            .path()
            .join(".backlog/tasks/task-0002 - review-request-2026-08-20-2.md")
            .exists());
        assert!(dir
            .path()
            .join(".backlog/tasks/task-0001 - review-request-2026-08-20-1.md")
            .exists());
    }

    #[test]
    fn missing_backlog_tree_errors_with_hint() {
        let dir = tempfile::tempdir().expect("tempdir");
        let registry = registry_with(sample_payload());
        let (_out, result) = run(&dir, &registry, RunMode::Write);
        let err = result.expect_err("must fail");
        assert!(
            err.to_string().contains("backlog init"),
            "error must hint at backlog init, got: {err:#}"
        );
    }

    #[test]
    fn missing_provider_errors_naming_the_provider() {
        let dir = scratch_backlog();
        let registry = DataRegistry::new();
        let (_out, result) = run(&dir, &registry, RunMode::Write);
        let err = result.expect_err("must fail");
        let rendered = format!("{err:#}");
        assert!(
            rendered.contains("no review_targets provider registered"),
            "error must name the missing provider, got: {rendered}"
        );
    }

    #[test]
    fn empty_targets_is_an_error() {
        let dir = scratch_backlog();
        let registry = registry_with(serde_json::json!({
            "skill": "code-review-rust",
            "targets": []
        }));
        let (_out, result) = run(&dir, &registry, RunMode::Write);
        let err = result.expect_err("must fail");
        assert!(
            err.to_string().contains("no targets"),
            "error must explain the empty target list, got: {err:#}"
        );
    }

    /// A payload that does not match the contract must surface as a decoding
    /// error, not silently produce tasks with missing fields.
    #[test]
    fn malformed_payload_is_a_decoding_error() {
        let dir = scratch_backlog();
        let registry = registry_with(serde_json::json!({ "skill": "code-review-rust" }));
        let (_out, result) = run(&dir, &registry, RunMode::Write);
        let err = result.expect_err("must fail");
        assert!(
            format!("{err:#}").contains("decoding review_targets payload"),
            "error must carry decoding context, got: {err:#}"
        );
    }

    /// Dry run reports the same lines as a real run with `would create`
    /// verbs, and leaves `.backlog/tasks` empty.
    #[test]
    fn dry_run_reports_without_writing() {
        let dir = scratch_backlog();
        let registry = registry_with(sample_payload());
        let (out, result) = run(&dir, &registry, RunMode::DryRun);
        result.expect("dry run must succeed");

        assert_eq!(
            out,
            concat!(
                "would create TASK-0001 review-request-2026-08-20-1 (2 subtasks)\n",
                "would create TASK-0001.01 REVIEW: Run skill code-review-rust against ops-core (crates/core)\n",
                "would create TASK-0001.02 REVIEW: Run skill code-review-rust against ops-cli (crates/cli)\n",
                "list subtasks: backlog task list --parent TASK-0001\n",
            )
        );
        let tasks_dir = dir.path().join(".backlog").join("tasks");
        assert_eq!(
            std::fs::read_dir(&tasks_dir).expect("tasks dir").count(),
            0,
            "dry run must not write any task file"
        );
    }

    /// Repeated dry runs must report the identical allocation: nothing was
    /// consumed, so the next dry run (and the eventual real run) still sees
    /// number 1 and sequence 1.
    #[test]
    fn repeated_dry_runs_report_identical_allocation() {
        let dir = scratch_backlog();
        let registry = registry_with(sample_payload());
        let (out1, result1) = run(&dir, &registry, RunMode::DryRun);
        result1.expect("first dry run");
        let (out2, result2) = run(&dir, &registry, RunMode::DryRun);
        result2.expect("second dry run");
        assert_eq!(out1, out2);
    }

    /// A task filename taken between the id scan and the create must send the
    /// run back to reallocate — never truncate the winner's file — and must
    /// leave nothing behind from the attempt it abandoned. The pre-created
    /// `task-0001.02` file stands in for the file a concurrent run wrote
    /// after this one scanned the directory.
    #[test]
    fn taken_filename_reallocates_and_leaves_no_partial_set() {
        let dir = scratch_backlog();
        let tasks_dir = dir.path().join(".backlog").join("tasks");
        let squatter =
            tasks_dir.join("task-0001.02 - REVIEW-Run-skill-code-review-rust-against-ops-cli.md");
        std::fs::write(&squatter, "not ours").expect("squatter file");

        let registry = registry_with(sample_payload());
        let (out, result) = run(&dir, &registry, RunMode::Write);
        result.expect("run must reallocate around the taken name");

        assert_eq!(
            std::fs::read_to_string(&squatter).expect("squatter"),
            "not ours",
            "the foreign file must not be truncated"
        );
        assert!(
            // The squatter is not a `review-request-` task, so it advances the
            // main-task number but not the per-day sequence.
            out.starts_with("created TASK-0002 review-request-2026-08-20-1 (2 subtasks)\n"),
            "got: {out}"
        );
        assert!(
            !tasks_dir
                .join("task-0001 - review-request-2026-08-20-1.md")
                .exists(),
            "the abandoned attempt's main task must be rolled back"
        );
        assert!(
            !tasks_dir
                .join("task-0001.01 - REVIEW-Run-skill-code-review-rust-against-ops-core.md")
                .exists(),
            "the abandoned attempt's first subtask must be rolled back"
        );
    }

    /// The duplicate-sequence case the post-reservation re-check exists for,
    /// driven directly: a plan whose title is already claimed under a
    /// *different* main number lands on an unused filename, so `create_new`
    /// succeeds and only the re-check can reject it.
    ///
    /// It is driven at this level because the interleaving that produces such
    /// a plan — one id scan landing either side of a concurrent rollback —
    /// cannot be forced through the public entry point.
    #[test]
    fn write_task_set_rejects_a_claim_another_number_already_holds() {
        let dir = scratch_backlog();
        let tasks_dir = dir.path().join(".backlog").join("tasks");
        let foreign = tasks_dir.join("task-0009 - review-request-2026-08-20-1.md");
        std::fs::write(&foreign, "another run").expect("foreign file");

        let targets: ReviewTargets = serde_json::from_value(sample_payload()).expect("payload");
        let plan = TaskPlan {
            main_number: 1,
            main_id: "TASK-0001".to_string(),
            main_title: "review-request-2026-08-20-1".to_string(),
            subtasks: vec![PlannedSubtask {
                index: 1,
                id: "TASK-0001.01".to_string(),
                title: "REVIEW: Run skill code-review-rust against ops-core".to_string(),
                path: &targets.targets[0].path,
            }],
        };

        let committed = write_task_set(dir.path(), &plan, &fixed_stamp()).expect("no io error");
        assert!(
            !committed,
            "a title another main number already claims must be rejected"
        );
        assert_eq!(
            std::fs::read_dir(&tasks_dir).expect("tasks dir").count(),
            1,
            "the rejected attempt must roll back, leaving only the foreign file"
        );
        assert_eq!(
            std::fs::read_to_string(&foreign).expect("foreign"),
            "another run",
            "the foreign file must be untouched"
        );
    }

    /// A failing task-file write must roll the whole set back and report
    /// nothing: a review request whose subtasks stop short of the targets it
    /// names is worse than no request at all.
    ///
    /// The failure is injected with a target name at the payload validator's
    /// length bound — accepted at the boundary, but long enough that its
    /// subtask filename exceeds the filesystem's per-component limit — so the
    /// *second* subtask fails after the main task and the first are on disk.
    /// The generated name is `task-0001.02 - ` (15) + the slugged title
    /// prefix (42) + the target name + `.md` (3) = 260 bytes, past the
    /// 255-byte component limit every filesystem this runs on enforces.
    #[test]
    fn failed_write_rolls_back_the_whole_set_and_reports_nothing() {
        let dir = scratch_backlog();
        let registry = registry_with(serde_json::json!({
            "skill": "code-review-rust",
            "targets": [
                { "name": "ops-core", "path": "crates/core" },
                { "name": "x".repeat(MAX_NAME_CHARS), "path": "crates/too-long" }
            ]
        }));
        let (out, result) = run(&dir, &registry, RunMode::Write);
        let err = result.expect_err("the oversized filename must fail the run");
        assert!(
            format!("{err:#}").contains("creating "),
            "error must name the file it could not create, got: {err:#}"
        );

        assert_eq!(out, "", "a rolled-back run must report no created tasks");
        let tasks_dir = dir.path().join(".backlog").join("tasks");
        assert_eq!(
            std::fs::read_dir(&tasks_dir).expect("tasks dir").count(),
            0,
            "every staged task file must be removed"
        );
    }

    /// Concurrent runs against one backlog must each get their own main-task
    /// number and their own complete set of files — no truncation, no
    /// interleaved ids, no missing subtask.
    #[test]
    fn concurrent_runs_allocate_disjoint_task_sets() {
        const RUNS: usize = 8;

        let dir = scratch_backlog();
        let registry = registry_with(sample_payload());
        let outputs: Vec<String> = std::thread::scope(|scope| {
            // The collect is load-bearing: spawn all RUNS threads before
            // joining any, or the runs serialise and stop racing.
            #[allow(clippy::needless_collect)]
            let handles: Vec<_> = (0..RUNS)
                .map(|_| {
                    scope.spawn(|| {
                        let (out, result) = run(&dir, &registry, RunMode::Write);
                        result.expect("concurrent run must succeed");
                        out
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|handle| handle.join().expect("thread must not panic"))
                .collect()
        });

        /// `created TASK-0002 review-request-2026-08-20-1 (2 subtasks)` → the
        /// whitespace-separated field at `field`.
        fn reported(out: &str, field: usize) -> &str {
            out.split_whitespace()
                .nth(field)
                .expect("report names the id and title")
        }

        // Every run reported a distinct main id...
        let mut main_ids: Vec<&str> = outputs.iter().map(|out| reported(out, 1)).collect();
        main_ids.sort_unstable();
        main_ids.dedup();
        assert_eq!(main_ids.len(), RUNS, "main ids must be unique across runs");

        // ...and a distinct review-request title. This asserts the invariant
        // end to end; it does not reproduce the interleaving that breaks it —
        // `write_task_set_rejects_a_claim_another_number_already_holds` drives
        // that case directly.
        let mut titles: Vec<&str> = outputs.iter().map(|out| reported(out, 2)).collect();
        titles.sort_unstable();
        titles.dedup();
        assert_eq!(
            titles.len(),
            RUNS,
            "review-request titles must be unique across runs, got: {outputs:?}"
        );

        // ...and every reported file is on disk, complete and unmangled.
        let tasks_dir = dir.path().join(".backlog").join("tasks");
        assert_eq!(
            std::fs::read_dir(&tasks_dir).expect("tasks dir").count(),
            RUNS * 3,
            "each run must leave exactly one main task plus two subtasks"
        );
        for main_id in main_ids {
            let number = main_id
                .strip_prefix("TASK-")
                .expect("zero-padded main id")
                .to_string();
            let main = std::fs::read_dir(&tasks_dir)
                .expect("tasks dir")
                .flatten()
                .map(|entry| entry.file_name().into_string().expect("utf8 name"))
                .filter(|name| name.starts_with(&format!("task-{number} ")))
                .count();
            assert_eq!(main, 1, "exactly one main task file for {main_id}");
            let subtasks = std::fs::read_dir(&tasks_dir)
                .expect("tasks dir")
                .flatten()
                .map(|entry| entry.file_name().into_string().expect("utf8 name"))
                .filter(|name| name.starts_with(&format!("task-{number}.")))
                .count();
            assert_eq!(subtasks, 2, "both subtask files present for {main_id}");
        }
    }

    /// A dry run after a real run previews the *next* allocation, mirroring
    /// what the next real run would create.
    #[test]
    fn dry_run_after_real_run_previews_next_allocation() {
        let dir = scratch_backlog();
        let registry = registry_with(sample_payload());
        let (_out, result) = run(&dir, &registry, RunMode::Write);
        result.expect("real run");
        let (out, result) = run(&dir, &registry, RunMode::DryRun);
        result.expect("dry run");
        assert!(
            out.starts_with("would create TASK-0002 review-request-2026-08-20-2 (2 subtasks)\n"),
            "got: {out}"
        );
    }

    /// ERR-6: an unreadable clock aborts the run in both modes, naming the
    /// clock and leaving the backlog untouched — never a task set dated
    /// 1970-01-01. The failure is driven through the real `UtcStamp` path,
    /// not a stubbed error.
    #[test]
    fn unreadable_clock_aborts_the_run_and_writes_nothing() {
        for mode in [RunMode::Write, RunMode::DryRun] {
            let dir = scratch_backlog();
            let registry = registry_with(sample_payload());
            let mut out = Vec::new();
            let pre_epoch = std::time::UNIX_EPOCH
                .checked_sub(std::time::Duration::from_secs(1))
                .expect("a pre-epoch SystemTime must be constructible");
            let result =
                run_create_review_tasks_with_clock(&registry, dir.path(), &mut out, mode, || {
                    UtcStamp::at(pre_epoch)
                });
            let err = result.expect_err("a pre-epoch clock must fail the run");
            let rendered = format!("{err:#}");
            assert!(
                rendered.contains("system clock reads before 1970-01-01"),
                "error must name the clock in {mode:?}, got: {rendered}"
            );
            assert!(out.is_empty(), "a failed clock read must report nothing");
            assert_eq!(
                std::fs::read_dir(dir.path().join(".backlog").join("tasks"))
                    .expect("tasks dir")
                    .count(),
                0,
                "no task file may be created in {mode:?}"
            );
        }
    }

    /// TEST-5: the public entry point — and with it `UtcStamp::now` — is
    /// exercised, not only its clock-injecting core.
    #[test]
    fn public_entry_point_writes_a_set_dated_by_the_host_clock() {
        let dir = scratch_backlog();
        let registry = registry_with(sample_payload());
        let mut out = Vec::new();
        run_create_review_tasks(&registry, dir.path(), &mut out, RunMode::Write)
            .expect("run must succeed");
        let report = String::from_utf8(out).expect("utf8");
        assert!(
            report.starts_with("created TASK-0001 review-request-"),
            "got: {report}"
        );
        let names: Vec<String> = std::fs::read_dir(dir.path().join(".backlog").join("tasks"))
            .expect("tasks dir")
            .flatten()
            .map(|entry| entry.file_name().into_string().expect("utf8 name"))
            .collect();
        assert_eq!(
            names.len(),
            3,
            "one main task plus two subtasks, got: {names:?}"
        );
        assert!(
            names
                .iter()
                .any(|name| name.starts_with("task-0001 - review-request-")),
            "got: {names:?}"
        );
    }

    /// TEST-6: the give-up bail, driven deterministically. A backlog whose
    /// highest main number *and* highest daily sequence are both `u32::MAX`
    /// saturates every allocation, so each attempt re-derives the same
    /// filename — the one the fixture already occupies — and loses the
    /// `create_new` race forever.
    #[test]
    fn allocation_gives_up_after_max_attempts_and_rolls_back() {
        const SQUATTER: &str = "task-4294967295 - review-request-2026-08-20-4294967295.md";

        let dir = scratch_backlog();
        let tasks_dir = dir.path().join(".backlog").join("tasks");
        std::fs::write(tasks_dir.join(SQUATTER), "another run").expect("squatter file");

        let registry = registry_with(sample_payload());
        let (out, result) = run(&dir, &registry, RunMode::Write);
        let err = result.expect_err("every attempt must lose the race");
        let rendered = format!("{err:#}");
        assert!(
            rendered.contains(&format!(
                "gave up allocating a free review-request id after {MAX_ALLOCATION_ATTEMPTS} attempts"
            )),
            "error must name the attempt count, got: {rendered}"
        );
        assert!(
            rendered.contains(&tasks_dir.display().to_string()),
            "error must name the tasks directory, got: {rendered}"
        );
        assert_eq!(out, "", "a run that never committed must report nothing");
        assert_eq!(
            std::fs::read_dir(&tasks_dir).expect("tasks dir").count(),
            1,
            "nothing from the final abandoned attempt may survive"
        );
        assert_eq!(
            std::fs::read_to_string(tasks_dir.join(SQUATTER)).expect("squatter"),
            "another run",
            "the foreign file must be untouched"
        );
    }

    /// SEC-11: a control character in a provider-supplied string is rejected
    /// at the boundary, before any file is written or any line reported.
    #[test]
    fn control_characters_in_the_payload_are_rejected() {
        let dir = scratch_backlog();
        let registry = registry_with(serde_json::json!({
            "skill": "code-review-rust",
            "targets": [{ "name": "ops\ncore", "path": "crates/core" }]
        }));
        let (out, result) = run(&dir, &registry, RunMode::Write);
        let err = result.expect_err("a newline in a target name must fail the run");
        let rendered = format!("{err:#}");
        assert!(
            rendered.contains("targets[1].name contains the control character"),
            "error must name the offending field, got: {rendered}"
        );
        assert!(
            !rendered.contains('\n'),
            "the offending value must be escaped, not echoed raw: {rendered}"
        );
        assert_eq!(out, "", "a rejected payload must report nothing");
        assert_eq!(
            std::fs::read_dir(dir.path().join(".backlog").join("tasks"))
                .expect("tasks dir")
                .count(),
            0,
            "no task file may be created"
        );
    }

    /// The other two boundary rules the payload validator enforces.
    #[test]
    fn empty_and_oversized_payload_fields_are_rejected() {
        let dir = scratch_backlog();
        let registry = registry_with(serde_json::json!({
            "skill": "",
            "targets": [{ "name": "ops-core", "path": "crates/core" }]
        }));
        let (_out, result) = run(&dir, &registry, RunMode::Write);
        let err = result.expect_err("an empty skill must fail the run");
        assert!(
            format!("{err:#}").contains("skill is empty"),
            "got: {err:#}"
        );

        let registry = registry_with(serde_json::json!({
            "skill": "code-review-rust",
            "targets": [{ "name": "x".repeat(MAX_NAME_CHARS + 1), "path": "crates/core" }]
        }));
        let (_out, result) = run(&dir, &registry, RunMode::Write);
        let err = result.expect_err("an over-long target name must fail the run");
        assert!(
            format!("{err:#}").contains("over the 200-character bound"),
            "got: {err:#}"
        );
    }
}
