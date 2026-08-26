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

/// Create the review-request task set: one main task plus one subtask per
/// review target, written as markdown files under `.backlog/tasks/`.
/// [`RunMode::DryRun`] prints the same report with `would create` verbs and
/// writes nothing.
///
/// # Errors
///
/// - No `.backlog/tasks` directory in `workspace_root` (hint: `backlog init`).
/// - No `review_targets` provider registered for the detected stack.
/// - The registered provider failed, or its payload did not match
///   [`ReviewTargets`].
/// - A task file could not be written (the error names the path;
///   [`RunMode::DryRun`] never takes this branch).
pub fn run_create_review_tasks(
    registry: &DataRegistry,
    workspace_root: &Path,
    out: &mut dyn Write,
    mode: RunMode,
) -> anyhow::Result<()> {
    run_create_review_tasks_at(registry, workspace_root, out, mode, &UtcStamp::now())
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

    let main_number = backlog::next_main_task_number(workspace_root);
    let sequence = backlog::next_daily_sequence(workspace_root, &stamp.date);
    let main_title = format!("review-request-{}-{sequence}", stamp.date);
    let main_id = backlog::main_task_id(main_number);
    let verb = match mode {
        RunMode::Write => "created",
        RunMode::DryRun => "would create",
    };

    let main_task = MainTaskFile {
        number: main_number,
        id: &main_id,
        title: &main_title,
    };
    write_main_task(workspace_root, &main_task, stamp, mode)?;
    writeln!(
        out,
        "{verb} {main_id} {main_title} ({} subtasks)",
        targets.targets.len()
    )?;

    for (index, target) in targets.targets.iter().enumerate() {
        let subtask_index = index + 1;
        let title = format!(
            "REVIEW: Run skill {} against {}",
            targets.skill, target.name
        );
        let subtask_id = backlog::subtask_id(main_number, subtask_index);
        let subtask = SubtaskFile {
            main_number,
            index: subtask_index,
            main_id: &main_id,
            subtask_id: &subtask_id,
            title: &title,
        };
        write_subtask(workspace_root, &subtask, stamp, mode)?;
        writeln!(out, "{verb} {subtask_id} {title} ({})", target.path)?;
    }
    // Printed in both modes: even a dry run shows the id the eventual real
    // run will allocate, so the filter is usable as soon as the tasks exist.
    writeln!(out, "list subtasks: backlog task list --parent {main_id}")?;
    Ok(())
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
    serde_json::from_value(payload)
        .with_context(|| format!("decoding {DATA_PROVIDER_NAME} payload"))
}

/// Identity of the main task file. FN-3: grouped rather than passed as
/// positional parameters, mirroring [`SubtaskFile`].
struct MainTaskFile<'a> {
    /// Main-task number (id and filename share it).
    number: u32,
    /// Zero-padded id, e.g. `TASK-0042`.
    id: &'a str,
    /// Main-task title.
    title: &'a str,
}

/// Render and write the main task file. ERR-13: the write error names the
/// exact file path. [`RunMode::DryRun`] returns before any filesystem touch.
fn write_main_task(
    workspace_root: &Path,
    main: &MainTaskFile<'_>,
    stamp: &UtcStamp,
    mode: RunMode,
) -> anyhow::Result<()> {
    if mode == RunMode::DryRun {
        return Ok(());
    }
    let file_name = backlog::main_task_file_name(main.number, main.title);
    let path = workspace_root
        .join(".backlog")
        .join("tasks")
        .join(&file_name);
    let mut file =
        std::fs::File::create(&path).with_context(|| format!("creating {}", path.display()))?;
    backlog::render_task_file(&mut file, main.id, main.title, stamp, None)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// Render and write one subtask file. FN-3: the identity fields travel as
/// one struct rather than six positional parameters.
struct SubtaskFile<'a> {
    /// Parent main-task number (id and filename share it).
    main_number: u32,
    /// 1-based position among the sibling subtasks.
    index: usize,
    /// Parent's zero-padded id, e.g. `TASK-0042`.
    main_id: &'a str,
    /// This subtask's dotted id, e.g. `TASK-0042.03`.
    subtask_id: &'a str,
    /// Subtask title.
    title: &'a str,
}

/// Render and write one subtask file. ERR-13: the write error names the
/// exact file path. [`RunMode::DryRun`] returns before any filesystem touch.
fn write_subtask(
    workspace_root: &Path,
    subtask: &SubtaskFile<'_>,
    stamp: &UtcStamp,
    mode: RunMode,
) -> anyhow::Result<()> {
    if mode == RunMode::DryRun {
        return Ok(());
    }
    let SubtaskFile {
        main_number,
        index,
        main_id,
        subtask_id,
        title,
    } = subtask;
    let file_name = backlog::subtask_file_name(*main_number, *index, title);
    let path = workspace_root
        .join(".backlog")
        .join("tasks")
        .join(&file_name);
    let mut file =
        std::fs::File::create(&path).with_context(|| format!("creating {}", path.display()))?;
    backlog::render_task_file(&mut file, subtask_id, title, stamp, Some((main_id, *index)))
        .with_context(|| format!("writing {}", path.display()))?;
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
}
