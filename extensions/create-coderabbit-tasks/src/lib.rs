//! Engine for `ops backlog create-coderabbit-tasks`: a CodeRabbit review of
//! the *whole* content of each review target, not just a diff.
//!
//! CodeRabbit's CLI only reviews changes, so a review target is made to look
//! new: in a throwaway worktree switched to an orphan branch whose only
//! commit is empty ([`worktree`]), every file is untracked and
//! `coderabbit review --uncommitted --include-untracked --dir <crate>` covers
//! all of it.
//!
//! Two modes share the `review_targets` provider of `create-review-tasks`:
//!
//! - without `--run`, [`CODERABBIT_REQUEST`] files a
//!   `coderabbit-review-request-<date>-<n>` main task plus one subtask per
//!   target whose body spells out the steps, for an agent or a person to run;
//! - with `--run`, [`run_coderabbit_reviews`] does those steps per target and
//!   files one Triage task per finding ([`finding`]) in the main checkout's
//!   backlog, then commits them.

// This crate holds no `unsafe` and must stay that way. `forbid` rather than
// the workspace's inherited `deny` so a later scoped `#[allow(unsafe_code)]`
// cannot lift it.
#![forbid(unsafe_code)]

mod finding;
mod worktree;

use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Context as _;
use ops_backlog::clock::UtcStamp;
use ops_backlog::cmd::{CommitOptions, Created, create_task, git, run_commit};
use ops_backlog::config::BacklogConfig;
use ops_backlog::store::Store;
use ops_create_review_tasks::{
    RequestScheme, ReviewTarget, RunMode, fetch_review_targets, select_targets,
};
use ops_extension::DataRegistry;

use crate::worktree::{ReviewWorktree, branch_name, slug};

/// The default program name, resolved on `PATH`.
pub const CODERABBIT: &str = "coderabbit";

/// The request scheme of `create-coderabbit-tasks` without `--run`: one
/// `REVIEW: Run CodeRabbit full review against <target>` subtask per target,
/// its body the steps `--run` automates.
pub const CODERABBIT_REQUEST: RequestScheme = RequestScheme {
    stem: "coderabbit-review-request",
    main_labels: &["code-review-request", "code-review", "coderabbit"],
    subtask_labels: &["code-review", "coderabbit"],
    subtask_title: |_skill, target| {
        format!("REVIEW: Run CodeRabbit full review against {}", target.name)
    },
    subtask_description: request_steps,
};

/// The subtask body: the worktree / orphan-branch steps for one target.
fn request_steps(target: &ReviewTarget) -> String {
    let name = &target.name;
    let path = &target.path;
    let branch = branch_name(name);
    let dir = format!("\"${{TMPDIR:-/tmp}}/ops-cr-{}\"", slug(name));
    format!(
        "Review the **whole** of `{name}` (`{path}`) with CodeRabbit, not just a diff. \
         CodeRabbit only reviews changes, so every file is made to look new: an orphan \
         branch with an empty commit, in a throwaway worktree.\n\n\
         `ops backlog create-coderabbit-tasks --run --crate {name}` runs every step below.\n\n\
         By hand, from the workspace root:\n\n\
         1. `git worktree add --detach {dir} HEAD`, then `cd {dir}`.\n\
         2. `git checkout --orphan {branch}`, then `git rm -r --cached -q .` (files stay on \
         disk, untracked), then `git commit --allow-empty --no-verify -m \"empty base\"`.\n\
         3. `coderabbit review --agent --uncommitted --include-untracked --dir \"$PWD/{path}\" \
         -c AGENTS.md` (add `--fresh` or `--use-credits` as needed).\n\
         4. `coderabbit review --show-prompts` prints the prompts of that review.\n\
         5. Back in the **main checkout**, file each finding: `ops backlog task create \
         \"CodeRabbit <severity>: <summary>\" -s Triage -l coderabbit,code-review \
         --modified-file <file> --unless-exists coderabbit:{name}:<file>:<hash>`.\n\
         6. Commit them: `ops backlog commit <ids> -m \"chore(backlog): file CodeRabbit \
         findings for {name}\"`.\n\
         7. Clean up: `git worktree remove --force {dir}` and `git branch -D {branch}`."
    )
}

/// What one `--run` invocation reviews and how.
#[derive(Debug, Clone)]
pub struct RunOptions<'a> {
    /// Target names to review; empty reviews every target.
    pub only: &'a [String],
    /// Pass `--fresh` to `coderabbit review`.
    pub fresh: bool,
    /// Pass `--use-credits` to `coderabbit review`.
    pub use_credits: bool,
    /// [`RunMode::DryRun`] previews the targets without creating a worktree
    /// or calling coderabbit.
    pub mode: RunMode,
    /// The coderabbit program; [`CODERABBIT`] resolves it on `PATH`.
    pub program: &'a OsStr,
}

/// The main checkout's backlog the findings are filed in.
#[derive(Debug, Clone, Copy)]
pub struct Backlog<'a> {
    /// The opened `.backlog` store.
    pub store: &'a Store,
    /// Its effective configuration.
    pub cfg: &'a BacklogConfig,
}

/// Review each selected target with CodeRabbit and file its findings.
///
/// Per target: add the review worktree, run `coderabbit review --agent`, file
/// one keyed Triage task per finding in `backlog`, commit the new task files
/// on the current branch, and remove the worktree and branch again — also
/// when a step fails or the run is interrupted (Ctrl-C reaches the
/// coderabbit child, and this process unwinds instead of dying).
///
/// # Errors
///
/// The provider fails or a `--crate` name is unknown; the index already holds
/// staged changes; coderabbit is missing, unauthenticated or over quota (all
/// checked before any worktree exists); a git step, the review, its output,
/// a task write or the commit fails; or the run was interrupted.
pub fn run_coderabbit_reviews(
    registry: &DataRegistry,
    backlog: Backlog<'_>,
    workspace_root: &Path,
    opts: &RunOptions<'_>,
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    let targets = select_targets(fetch_review_targets(registry, workspace_root)?, opts.only)?;
    if opts.mode == RunMode::DryRun {
        for target in &targets.targets {
            writeln!(
                out,
                "would review {} ({}) with coderabbit in a throwaway worktree on branch {}",
                target.name,
                target.path,
                branch_name(&target.name)
            )?;
        }
        return Ok(());
    }

    let top = git::toplevel(workspace_root)?;
    let staged = git::staged_paths(&top)?;
    if !staged.is_empty() {
        anyhow::bail!(
            "the index already holds staged changes ({}) — commit or unstage them first, so \
             the findings commit holds only the new task files",
            staged.join(", ")
        );
    }
    preflight(opts.program, workspace_root)?;

    // The workspace root's place inside the repository, so the same
    // directory can be found inside each worktree.
    let root = workspace_root
        .canonicalize()
        .with_context(|| format!("resolving {}", workspace_root.display()))?;
    let within = root.strip_prefix(&top).unwrap_or_else(|_| Path::new("")).to_path_buf();
    // Held exactly across the review loop: dropping it at the end of this
    // function restores the default SIGINT/SIGTERM disposition.
    let interrupt = InterruptGuard::install()?;
    let run = TargetRun {
        backlog,
        workspace_root,
        top: &top,
        within: &within,
        opts,
        interrupted: interrupt.flag(),
    };
    for target in &targets.targets {
        run.review(target, out)?;
    }
    Ok(())
}

/// Fail before any worktree exists when coderabbit cannot run a review:
/// missing from `PATH`, not logged in, or out of quota.
fn preflight(program: &OsStr, cwd: &Path) -> anyhow::Result<()> {
    let check = |args: &[&str], what: &str, hint: &str| -> anyhow::Result<()> {
        let output = Command::new(program)
            .args(args)
            .current_dir(cwd)
            .stdin(Stdio::null())
            .output()
            .map_err(|err| {
                if err.kind() == std::io::ErrorKind::NotFound {
                    anyhow::anyhow!(
                        "{} not found on PATH — install the CodeRabbit CLI and run \
                         `coderabbit auth login`",
                        program.to_string_lossy()
                    )
                } else {
                    anyhow::Error::new(err)
                        .context(format!("running coderabbit {}", args.join(" ")))
                }
            })?;
        if !output.status.success() {
            anyhow::bail!("{what} ({}): {} — {hint}", output.status, display_output(&output));
        }
        Ok(())
    };
    check(&["auth", "status"], "coderabbit is not authenticated", "run `coderabbit auth login`")?;
    check(
        &["usage"],
        "coderabbit usage check failed",
        "the account may be over its review quota; see `coderabbit usage`",
    )
}

/// A child's stderr (or, when empty, stdout) as one escaped, bounded line
/// for an error message: it is another program's output, so its control
/// characters are escaped rather than echoed to the terminal.
fn display_output(output: &std::process::Output) -> String {
    const MAX_CHARS: usize = 400;
    let stderr = String::from_utf8_lossy(&output.stderr);
    let text = if stderr.trim().is_empty() {
        String::from_utf8_lossy(&output.stdout).into_owned()
    } else {
        stderr.into_owned()
    };
    let text: String = text.trim().chars().take(MAX_CHARS).collect();
    format!("{text:?}")
}

/// The SIGINT/SIGTERM handlers of one run and the flag they set.
///
/// While it lives, the first signal no longer kills this process: the
/// coderabbit child (same process group) still gets it and exits, and the run
/// unwinds through the worktree guard. A second signal takes the default
/// action, so a wedged cleanup can still be killed. Dropping it unregisters
/// every handler it installed, so the process's signal disposition is
/// restored once the review loop ends.
struct InterruptGuard {
    flag: Arc<AtomicBool>,
    ids: Vec<signal_hook::SigId>,
}

impl InterruptGuard {
    fn install() -> anyhow::Result<Self> {
        use signal_hook::consts::{SIGINT, SIGTERM};
        // Built first, so a registration that fails half-way still releases
        // the handlers already installed.
        let mut guard = Self { flag: Arc::new(AtomicBool::new(false)), ids: Vec::new() };
        for signal in [SIGINT, SIGTERM] {
            // Registered first, so it sees the flag as it was before this
            // signal: set means this is the second one.
            guard.ids.push(
                signal_hook::flag::register_conditional_default(signal, Arc::clone(&guard.flag))
                    .context("installing the interrupt handler")?,
            );
            guard.ids.push(
                signal_hook::flag::register(signal, Arc::clone(&guard.flag))
                    .context("installing the interrupt handler")?,
            );
        }
        Ok(guard)
    }

    /// Set once SIGINT or SIGTERM arrived while the guard was installed.
    fn flag(&self) -> &AtomicBool {
        &self.flag
    }
}

impl Drop for InterruptGuard {
    fn drop(&mut self) {
        for id in self.ids.drain(..) {
            signal_hook::low_level::unregister(id);
        }
    }
}

/// The tally of one review's findings, kept in the caller so the ids filed
/// before a failure are still known to it.
#[derive(Debug, Default)]
struct Filing {
    /// Ids of the task files written by this review.
    filed: Vec<String>,
    /// Findings whose task already existed.
    existing: usize,
    /// Findings skipped because their file is outside the repository.
    skipped: usize,
}

/// Where one review's findings are filed: the backlog, the target they are
/// keyed under, the worktree root their paths are relative to, and the
/// creation stamp they share.
struct Filer<'a> {
    backlog: Backlog<'a>,
    target_name: &'a str,
    worktree_root: &'a Path,
    stamp: &'a UtcStamp,
}

/// File one Triage task per finding, recording each outcome in `filing`.
///
/// A finding whose file is refused by [`finding::repo_relative_file`] is
/// skipped with a warning on `out`, so one malformed finding does not cost the
/// findings after it.
///
/// # Errors
///
/// A task write or a write to `out` fails; `filing` still holds every task
/// filed before it.
fn file_findings(
    filer: &Filer<'_>,
    findings: &[finding::Finding],
    filing: &mut Filing,
    out: &mut dyn Write,
) -> anyhow::Result<()> {
    for item in findings {
        let file = match finding::repo_relative_file(&item.file_name, filer.worktree_root) {
            Ok(file) => file,
            Err(err) => {
                writeln!(out, "skipped: {err}")?;
                filing.skipped = filing.skipped.saturating_add(1);
                continue;
            }
        };
        let opts = finding::task_options(filer.target_name, &file, item);
        match create_task(filer.backlog.store, filer.backlog.cfg, &opts, filer.stamp)? {
            Created::New(id) => {
                writeln!(out, "created {id} {}", opts.title)?;
                filing.filed.push(id);
            }
            Created::Exists(id) => {
                writeln!(out, "exists {id} {}", opts.title)?;
                filing.existing = filing.existing.saturating_add(1);
            }
        }
    }
    Ok(())
}

/// What every target's review shares.
struct TargetRun<'a> {
    backlog: Backlog<'a>,
    workspace_root: &'a Path,
    /// Top level of the main checkout.
    top: &'a Path,
    /// The workspace root relative to `top`.
    within: &'a Path,
    opts: &'a RunOptions<'a>,
    interrupted: &'a AtomicBool,
}

impl TargetRun<'_> {
    /// Fail if a signal arrived since the last check.
    fn check_interrupted(&self) -> anyhow::Result<()> {
        if self.interrupted.load(Ordering::SeqCst) {
            anyhow::bail!("interrupted — the review worktree and branch were removed");
        }
        Ok(())
    }

    /// Review one target, file and commit its findings. The worktree guard
    /// lives exactly as long as this call.
    fn review(&self, target: &ReviewTarget, out: &mut dyn Write) -> anyhow::Result<()> {
        self.check_interrupted()?;
        writeln!(out, "reviewing {} ({}) with coderabbit", target.name, target.path)?;
        let worktree = ReviewWorktree::create(self.top, &target.name)?;
        self.check_interrupted()?;
        let stdout = self.run_coderabbit(&worktree, target)?;
        self.check_interrupted()?;
        let findings = finding::parse_agent_output(&stdout)
            .with_context(|| format!("reading the coderabbit review of {}", target.name))?;

        // One clock read per review: every finding it files shares the stamp.
        let stamp = UtcStamp::now()?;
        let mut filing = Filing::default();
        let filer = Filer {
            backlog: self.backlog,
            target_name: &target.name,
            worktree_root: worktree.path(),
            stamp: &stamp,
        };
        let filed = file_findings(&filer, &findings, &mut filing, out);
        // Commit whatever was filed even when filing stopped early: a task
        // file left uncommitted is never picked up again, since a re-run
        // reports it as already filed.
        let committed = self.commit_filed(&target.name, &filing.filed, out);
        match (filed, committed) {
            (Ok(()), committed) => committed?,
            (Err(err), Ok(())) if !filing.filed.is_empty() => {
                return Err(err.context(format!(
                    "filing the coderabbit findings of {} stopped; committed the ones filed so \
                     far: {}",
                    target.name,
                    filing.filed.join(" ")
                )));
            }
            (Err(err), Ok(())) => return Err(err),
            // Both failed: the filing error is the cause, the commit error
            // names the uncommitted ids and the command that commits them.
            (Err(err), Err(commit)) => return Err(err.context(format!("{commit:#}"))),
        }
        writeln!(
            out,
            "{}: {} finding(s), {} new, {} already filed, {} skipped",
            target.name,
            findings.len(),
            filing.filed.len(),
            filing.existing,
            filing.skipped
        )?;
        Ok(())
    }

    /// Commit the task files `filed` created; a failure names them and the
    /// command that commits them by hand.
    fn commit_filed(
        &self,
        name: &str,
        filed: &[String],
        out: &mut dyn Write,
    ) -> anyhow::Result<()> {
        if filed.is_empty() {
            return Ok(());
        }
        let commit = CommitOptions {
            task_ids: filed.to_vec(),
            message: format!("chore(backlog): file CodeRabbit findings for {name}"),
        };
        run_commit(self.backlog.store, self.workspace_root, &commit, &mut &mut *out).with_context(
            || {
                format!(
                    "committing the new task files {ids} — commit them with `ops backlog commit \
                     {ids} -m \"{msg}\"`",
                    ids = filed.join(" "),
                    msg = commit.message
                )
            },
        )
    }

    /// Run `coderabbit review --agent` over the target's directory in the
    /// worktree and return its stdout; its stderr (progress) passes through.
    fn run_coderabbit(
        &self,
        worktree: &ReviewWorktree,
        target: &ReviewTarget,
    ) -> anyhow::Result<String> {
        let workspace = worktree.path().join(self.within);
        let crate_dir: PathBuf = workspace.join(&target.path);
        let mut command = Command::new(self.opts.program);
        command
            .current_dir(&workspace)
            .args(["review", "--agent", "--uncommitted", "--include-untracked", "--dir"])
            .arg(&crate_dir);
        let agents = workspace.join("AGENTS.md");
        if agents.is_file() {
            command.arg("-c").arg(&agents);
        }
        if self.opts.fresh {
            command.arg("--fresh");
        }
        if self.opts.use_credits {
            command.arg("--use-credits");
        }
        let output = command
            // The worktree, not an inherited hook environment, decides the
            // repository coderabbit reads.
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .stdin(Stdio::null())
            .stderr(Stdio::inherit())
            .output()
            .with_context(|| format!("running coderabbit review for {}", target.name))?;
        self.check_interrupted()?;
        if !output.status.success() {
            anyhow::bail!(
                "coderabbit review of {} failed ({}): {}",
                target.name,
                output.status,
                display_output(&output)
            );
        }
        String::from_utf8(output.stdout)
            .with_context(|| format!("coderabbit review of {} printed non-UTF-8", target.name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preflight_names_a_missing_program() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err =
            preflight(OsStr::new("ops-no-such-coderabbit"), dir.path()).expect_err("must fail");
        assert!(err.to_string().contains("not found on PATH"), "{err}");
    }

    #[test]
    fn an_invalid_file_name_skips_only_its_own_finding() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".backlog/tasks")).expect("tasks");
        let store = Store::open(&dir.path().join(".backlog")).expect("open");
        let cfg = BacklogConfig::default();
        let stream = concat!(
            "{\"type\":\"finding\",\"severity\":\"minor\",\"fileName\":\"/tmp/wt/a.rs\",\"comment\":\"First.\"}\n",
            "{\"type\":\"finding\",\"severity\":\"minor\",\"fileName\":\"/tmp/wt2/b.rs\",\"comment\":\"Outside.\"}\n",
            "{\"type\":\"finding\",\"severity\":\"minor\",\"fileName\":\"c.rs\",\"comment\":\"Third.\"}\n",
            "{\"type\":\"complete\"}\n",
        );
        let findings = finding::parse_agent_output(stream).expect("parses");
        let mut filing = Filing::default();
        let mut out = Vec::new();
        let stamp = UtcStamp::now().expect("clock");
        let filer = Filer {
            backlog: Backlog { store: &store, cfg: &cfg },
            target_name: "ops-core",
            worktree_root: Path::new("/tmp/wt"),
            stamp: &stamp,
        };
        file_findings(&filer, &findings, &mut filing, &mut out).expect("files the valid findings");
        let out = String::from_utf8(out).expect("utf8");
        assert_eq!(filing.filed.len(), 2, "{out}");
        assert_eq!((filing.existing, filing.skipped), (0, 1), "{out}");
        assert!(out.contains("skipped: coderabbit named a file outside the repository"), "{out}");
        assert!(out.contains("/tmp/wt2/b.rs"), "{out}");
    }

    #[test]
    fn request_steps_spell_out_the_orphan_worktree_review() {
        let steps = request_steps(&ReviewTarget {
            name: "ops-core".to_string(),
            path: "crates/core".to_string(),
        });
        for expected in [
            "--run --crate ops-core",
            "git worktree add --detach \"${TMPDIR:-/tmp}/ops-cr-ops-core\" HEAD",
            "git checkout --orphan cr-full-ops-core",
            "git rm -r --cached -q .",
            "--dir \"$PWD/crates/core\" -c AGENTS.md",
            "coderabbit review --show-prompts",
            "--unless-exists coderabbit:ops-core:<file>:<hash>",
            "git branch -D cr-full-ops-core",
        ] {
            assert!(steps.contains(expected), "missing {expected:?} in:\n{steps}");
        }
    }
}
