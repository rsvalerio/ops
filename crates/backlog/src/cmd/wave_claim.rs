//! `wave claim` / `wave park`: the wave runner's claim and park steps as
//! single operations.
//!
//! Claiming a wave is creating its branch and worktree; `git worktree add
//! -b` refusing an existing branch is what makes the claim exclusive. Only
//! once that succeeded does the wave parent flip to In Progress — so a
//! refused claim never leaves a task marked in progress by a run that never
//! started. Parking is the opposite end: the merge did not land, so the
//! branch and worktree stay for resumption and the task records why.

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context as _;

use super::edit::{run_edit, EditOptions};
use super::git;
use crate::store::Store;

/// The branch a wave is claimed on unless `--branch` says otherwise.
fn default_branch(wave_id: &str) -> String {
    format!("code-review/{wave_id}")
}

/// Arguments of `wave claim`.
#[derive(Debug, Clone)]
pub struct WaveClaimOptions {
    pub wave_id: String,
    /// The marker label identifying a wave.
    pub marker: String,
    /// Branch to create; default `code-review/<wave-id>`.
    pub branch: Option<String>,
    /// Worktree path; default `<repo-parent>/.wave-<wave-id>`, a sibling of
    /// the repository so it never shows up as untracked files inside it.
    pub worktree: Option<PathBuf>,
}

/// Arguments of `wave park`.
#[derive(Debug, Clone)]
pub struct WaveParkOptions {
    pub wave_id: String,
    /// Why the wave was parked — recorded in the task notes.
    pub reason: String,
    /// Status to leave the wave in (`In Progress` or `To Do`).
    pub status: String,
    /// The wave's branch; default `code-review/<wave-id>`.
    pub branch: Option<String>,
}

/// Claim a wave: create its branch and worktree, then flip it to
/// In Progress and record the branch and worktree in its notes.
///
/// # Errors
///
/// The wave id resolves to nothing or to a task without the wave marker;
/// `cwd` is not in a git working tree; the branch already exists (the wave
/// is claimed elsewhere) or the worktree path is taken — both refused before
/// anything is created; `git worktree add` fails; or the status edit fails,
/// in which case the just-created worktree and branch are removed again
/// (without force) so the claim leaves no half state.
pub fn run_wave_claim<W: Write>(
    store: &Store,
    cwd: &Path,
    opts: &WaveClaimOptions,
    out: &mut W,
) -> anyhow::Result<()> {
    let wave = store
        .find(&opts.wave_id)?
        .ok_or_else(|| anyhow::anyhow!("task {} not found", opts.wave_id))?;
    if !super::wave::is_wave(&wave, &opts.marker) {
        anyhow::bail!(
            "{} is not a wave (no {} label)",
            wave.doc.frontmatter.id,
            opts.marker
        );
    }
    let wave_id = wave.doc.frontmatter.id;
    let top = git::toplevel(cwd)?;
    let branch = opts
        .branch
        .clone()
        .unwrap_or_else(|| default_branch(&wave_id));
    if branch.starts_with('-') || branch.trim().is_empty() {
        anyhow::bail!("invalid branch name {branch:?}");
    }
    let worktree = match &opts.worktree {
        Some(path) if path.is_absolute() => path.clone(),
        Some(path) => cwd.join(path),
        None => top
            .parent()
            .unwrap_or(&top)
            .join(format!(".wave-{wave_id}")),
    };

    if git::branch_exists(&top, &branch)? {
        anyhow::bail!(
            "{wave_id} is already claimed: branch {branch} exists — pick another wave, \
             or resume the parked one from its worktree"
        );
    }
    if worktree
        .try_exists()
        .with_context(|| format!("checking {}", worktree.display()))?
    {
        anyhow::bail!(
            "{} already exists; refusing to claim {wave_id} over it",
            worktree.display()
        );
    }

    let worktree_arg = worktree.as_os_str().to_os_string();
    git::run(
        &top,
        [
            std::ffi::OsString::from("worktree"),
            "add".into(),
            "-b".into(),
            branch.clone().into(),
            worktree_arg,
        ],
    )
    .with_context(|| format!("claiming {wave_id}"))?;

    let edit = EditOptions {
        task_id: wave_id.clone(),
        status: Some("In Progress".to_string()),
        append_notes: vec![format!(
            "Branch: {branch}\nWorktree: {}",
            worktree.display()
        )],
        ..EditOptions::default()
    };
    if let Err(err) = run_edit(store, &edit, &mut std::io::sink()) {
        // Undo the claim so a retry starts clean. Neither step forces: the
        // fresh worktree holds nothing, and a refusal here means someone
        // already wrote into it, which must not be discarded.
        let undo = git::run(
            &top,
            [
                std::ffi::OsString::from("worktree"),
                "remove".into(),
                worktree.as_os_str().to_os_string(),
            ],
        )
        .and_then(|_| git::run(&top, ["branch", "-d", branch.as_str()]));
        return Err(match undo {
            Ok(_) => err.context(format!(
                "claiming {wave_id}: the status edit failed; branch and worktree removed again"
            )),
            Err(undo_err) => err.context(format!(
                "claiming {wave_id}: the status edit failed and undoing the claim failed too \
                 ({undo_err:#}); branch {branch} and worktree {} are left in place",
                worktree.display()
            )),
        });
    }

    writeln!(
        out,
        "Claimed {wave_id}: branch {branch}, worktree {}",
        worktree.display()
    )
    .context("printing the claim summary")?;
    Ok(())
}

/// Park a wave: set its status and record why it did not land, keeping its
/// branch and worktree for resumption. Nothing in git is touched.
///
/// # Errors
///
/// The reason or status is blank; the wave id resolves to nothing; `cwd` is
/// not in a git working tree; or the task edit fails.
pub fn run_wave_park<W: Write>(
    store: &Store,
    cwd: &Path,
    opts: &WaveParkOptions,
    out: &mut W,
) -> anyhow::Result<()> {
    if opts.reason.trim().is_empty() {
        anyhow::bail!("a park reason is required");
    }
    if opts.status.trim().is_empty() {
        anyhow::bail!("the park status is empty");
    }
    let wave = store
        .find(&opts.wave_id)?
        .ok_or_else(|| anyhow::anyhow!("task {} not found", opts.wave_id))?;
    let wave_id = wave.doc.frontmatter.id;
    let top = git::toplevel(cwd)?;
    let branch = opts
        .branch
        .clone()
        .unwrap_or_else(|| default_branch(&wave_id));

    let resume = if git::branch_exists(&top, &branch)? {
        worktree_of(&top, &branch)?.map_or_else(
            || format!("Resume from branch {branch} (no worktree checked out)"),
            |path| format!("Resume from branch {branch}, worktree {}", path.display()),
        )
    } else {
        format!("Branch {branch} does not exist; nothing to resume")
    };
    let edit = EditOptions {
        task_id: wave_id.clone(),
        status: Some(opts.status.clone()),
        append_notes: vec![format!("Parked: {}\n{resume}", opts.reason.trim())],
        ..EditOptions::default()
    };
    run_edit(store, &edit, &mut std::io::sink())?;
    writeln!(out, "Parked {wave_id} ({}): {resume}", opts.status)
        .context("printing the park summary")?;
    Ok(())
}

/// The worktree that has `branch` checked out, if any.
///
/// # Errors
///
/// `git worktree list` failed.
fn worktree_of(top: &Path, branch: &str) -> anyhow::Result<Option<PathBuf>> {
    let listing = git::run(top, ["worktree", "list", "--porcelain", "-z"])?;
    let wanted = format!("branch refs/heads/{branch}");
    let mut current: Option<PathBuf> = None;
    for record in listing.split('\0') {
        if let Some(path) = record.strip_prefix("worktree ") {
            current = Some(PathBuf::from(path));
        } else if record == wanted {
            return Ok(current);
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::cmd::git::test_support::git;
    use crate::model::TaskDoc;

    fn wave_task(id: &str) -> String {
        format!(
            "---\nid: {id}\ntitle: 'wave {id}'\nstatus: To Do\nassignee: []\n\
             created_date: '2026-01-01 00:00'\nlabels:\n  - code-review-wave\n\
             dependencies: []\n---\n"
        )
    }

    /// `<tmp>/repo` holding one committed wave TASK-1; worktrees default to
    /// `<tmp>/.wave-TASK-1`, inside the tempdir.
    fn repo() -> (tempfile::TempDir, PathBuf, Store) {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("repo");
        std::fs::create_dir_all(root.join(".backlog/tasks")).expect("tasks");
        crate::cmd::git::test_support::init_repo(&root);
        std::fs::write(
            root.join(".backlog/tasks/task-1 - wave.md"),
            wave_task("TASK-1"),
        )
        .expect("seed");
        git(&root, &["add", "."]);
        git(&root, &["commit", "-q", "-m", "seed"]);
        let store = Store::open(&root.join(".backlog")).expect("open");
        (dir, root, store)
    }

    fn claim_opts() -> WaveClaimOptions {
        WaveClaimOptions {
            wave_id: "TASK-1".to_string(),
            marker: super::super::DEFAULT_WAVE_MARKER.to_string(),
            branch: None,
            worktree: None,
        }
    }

    fn wave_doc(root: &Path) -> TaskDoc {
        TaskDoc::parse(
            &std::fs::read_to_string(root.join(".backlog/tasks/task-1 - wave.md")).expect("read"),
        )
        .expect("parse")
    }

    /// AC #2: a successful claim leaves the branch, the worktree and the
    /// In Progress status (with the branch recorded) in place.
    #[test]
    fn claim_creates_branch_worktree_and_status() {
        let (dir, root, store) = repo();
        let mut out = Vec::new();
        run_wave_claim(&store, &root, &claim_opts(), &mut out).expect("claim");
        assert!(git::branch_exists(&root, "code-review/TASK-1").expect("show-ref"));
        assert!(dir.path().join(".wave-TASK-1/.git").exists());
        let doc = wave_doc(&root);
        assert_eq!(doc.frontmatter.status, "In Progress");
        let text =
            std::fs::read_to_string(root.join(".backlog/tasks/task-1 - wave.md")).expect("read");
        assert!(text.contains("Branch: code-review/TASK-1"), "got: {text}");
    }

    /// AC #1: an existing branch refuses the claim with no side effects —
    /// no worktree, the status untouched.
    #[test]
    fn claim_on_an_existing_branch_changes_nothing() {
        let (dir, root, store) = repo();
        git(&root, &["branch", "code-review/TASK-1"]);
        let before =
            std::fs::read_to_string(root.join(".backlog/tasks/task-1 - wave.md")).expect("read");
        let err =
            run_wave_claim(&store, &root, &claim_opts(), &mut Vec::new()).expect_err("must refuse");
        assert!(
            format!("{err:#}").contains("already claimed"),
            "got: {err:#}"
        );
        assert!(!dir.path().join(".wave-TASK-1").exists());
        assert_eq!(
            std::fs::read_to_string(root.join(".backlog/tasks/task-1 - wave.md")).expect("read"),
            before
        );
    }

    /// A second claim of the same wave is refused, as a concurrent runner's
    /// would be.
    #[test]
    fn a_second_claim_is_refused() {
        let (_dir, root, store) = repo();
        run_wave_claim(&store, &root, &claim_opts(), &mut Vec::new()).expect("first");
        let err =
            run_wave_claim(&store, &root, &claim_opts(), &mut Vec::new()).expect_err("second");
        assert!(
            format!("{err:#}").contains("already claimed"),
            "got: {err:#}"
        );
    }

    #[test]
    fn claim_refuses_a_task_that_is_not_a_wave() {
        let (_dir, root, store) = repo();
        std::fs::write(
            root.join(".backlog/tasks/task-2 - plain.md"),
            "---\nid: TASK-2\ntitle: plain\nstatus: To Do\nassignee: []\n\
             created_date: '2026-01-01 00:00'\nlabels: []\ndependencies: []\n---\n",
        )
        .expect("seed");
        let mut opts = claim_opts();
        opts.wave_id = "TASK-2".to_string();
        let err = run_wave_claim(&store, &root, &opts, &mut Vec::new()).expect_err("not a wave");
        assert!(format!("{err:#}").contains("not a wave"), "got: {err:#}");
        assert!(!git::branch_exists(&root, "code-review/TASK-2").expect("show-ref"));
    }

    /// AC #3: parking keeps the branch and worktree and records the reason
    /// and where to resume.
    #[test]
    fn park_keeps_branch_and_worktree_and_records_why() {
        let (dir, root, store) = repo();
        run_wave_claim(&store, &root, &claim_opts(), &mut Vec::new()).expect("claim");
        let mut out = Vec::new();
        run_wave_park(
            &store,
            &root,
            &WaveParkOptions {
                wave_id: "TASK-1".to_string(),
                reason: "integration verify failed".to_string(),
                status: "To Do".to_string(),
                branch: None,
            },
            &mut out,
        )
        .expect("park");
        assert!(git::branch_exists(&root, "code-review/TASK-1").expect("show-ref"));
        assert!(dir.path().join(".wave-TASK-1/.git").exists());
        assert_eq!(wave_doc(&root).frontmatter.status, "To Do");
        let text =
            std::fs::read_to_string(root.join(".backlog/tasks/task-1 - wave.md")).expect("read");
        assert!(
            text.contains("Parked: integration verify failed"),
            "got: {text}"
        );
        assert!(
            text.contains(".wave-TASK-1"),
            "resume path recorded: {text}"
        );
    }

    #[test]
    fn park_requires_a_reason() {
        let (_dir, root, store) = repo();
        let err = run_wave_park(
            &store,
            &root,
            &WaveParkOptions {
                wave_id: "TASK-1".to_string(),
                reason: "  ".to_string(),
                status: "In Progress".to_string(),
                branch: None,
            },
            &mut Vec::new(),
        )
        .expect_err("blank reason");
        assert!(format!("{err:#}").contains("reason"), "got: {err:#}");
    }
}
