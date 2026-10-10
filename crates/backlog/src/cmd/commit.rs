//! `commit`: commit exactly the named tasks' files, and nothing else — even
//! with other writers staging into the same index between the steps.
//!
//! 1. resolve each task id to its file;
//! 2. refuse — touching nothing — when the index already holds any other
//!    path;
//! 3. keep only the task files that actually changed, and refuse an empty
//!    commit;
//! 4. `git add` those files and `git commit --only -- <files>`.
//!
//! `--only` is what makes step 4 safe against a concurrent writer: it
//! commits exactly the named paths even if that writer stages something
//! between step 2 and step 4, and leaves the writer's entries staged for
//! its own commit.

use std::io::Write;
use std::path::Path;

use anyhow::Context as _;

use super::git;
use crate::model::TaskDoc;
use crate::store::Store;

/// Arguments of `commit`.
#[derive(Debug, Clone)]
pub struct CommitOptions {
    /// Task ids whose files may be committed.
    pub task_ids: Vec<String>,
    /// The commit message.
    pub message: String,
}

/// Commit the listed tasks' changed files as one commit.
///
/// `cwd` is any directory inside the git working tree that holds the
/// backlog.
///
/// # Errors
///
/// No task id or an empty message was given; a task id resolves to nothing;
/// `cwd` is not in a git working tree; the index already holds a path that
/// is not one of the tasks' files (nothing is touched); none of the tasks'
/// files changed (no commit is made); or a git step failed — each error names
/// what it refused or what failed.
pub fn run_commit<W: Write>(
    store: &Store,
    cwd: &Path,
    opts: &CommitOptions,
    out: &mut W,
) -> anyhow::Result<()> {
    if opts.task_ids.is_empty() {
        anyhow::bail!("no task ids given: name the tasks whose files to commit");
    }
    if opts.message.trim().is_empty() {
        anyhow::bail!("the commit message is empty");
    }
    let top = git::toplevel(cwd)?;
    let task_paths = resolve_task_paths(store, &top, &opts.task_ids)?;
    ensure_index_pure(&top, &task_paths)?;
    let changed = changed_task_paths(&top, &task_paths, &opts.task_ids)?;
    commit_changed_files(&top, &changed, &opts.message)?;
    print_commit_summary(&top, &changed, out)
}

/// Step 2: the index must hold nothing but our own files. Checked before
/// any mutation, so a refusal leaves the index exactly as it was.
///
/// # Errors
///
/// The index already holds a path that is not one of `task_paths` —
/// nothing is touched.
fn ensure_index_pure(top: &Path, task_paths: &[String]) -> anyhow::Result<()> {
    let foreign: Vec<String> = git::staged_paths(top)?
        .into_iter()
        .filter(|staged| !task_paths.contains(staged))
        .collect();
    if !foreign.is_empty() {
        anyhow::bail!(
            "refusing to commit: the index already holds paths that are not these tasks' \
             files: {} — unstage them (or commit them separately) and re-run",
            foreign.join(", ")
        );
    }
    Ok(())
}

/// Step 3: only the task files with a change (staged or not, including a
/// brand-new untracked file) are committed.
///
/// # Errors
///
/// None of `task_paths` changed (no commit is made).
fn changed_task_paths(
    top: &Path,
    task_paths: &[String],
    task_ids: &[String],
) -> anyhow::Result<Vec<String>> {
    let mut status_args: Vec<String> = [
        "status",
        "--porcelain=v1",
        "-z",
        // A staged rename would print as `R  new\0old`; keep every path its
        // own record so the old path reads as the deletion it is.
        "--no-renames",
        "--untracked-files=all",
        "--",
    ]
    .iter()
    .map(ToString::to_string)
    .collect();
    status_args.extend(task_paths.iter().cloned());
    let status = git::run(top, &status_args)?;
    let changed_in_status: Vec<String> = git::nul_records(&status)
        .into_iter()
        .filter_map(|record| record.get(3..).map(str::to_string))
        .collect();
    let changed: Vec<String> = task_paths
        .iter()
        .filter(|path| changed_in_status.contains(path))
        .cloned()
        .collect();
    if changed.is_empty() {
        anyhow::bail!(
            "nothing to commit: none of the files of {} changed",
            task_ids.join(", ")
        );
    }
    Ok(changed)
}

/// Step 4: stage the still-present changed files and commit exactly them.
///
/// `git add` is what brings a new untracked file in; a deleted path is left
/// to `commit --only`, which records the deletion whether or not it is
/// staged (`git add` of an already-staged deletion fails). A failed commit
/// puts the index back the way step 2 found it: none of our files were
/// staged before `git add` above.
///
/// # Errors
///
/// A git step failed; on a failed commit the staged files are reset again
/// before the error is returned.
fn commit_changed_files(top: &Path, changed: &[String], message: &str) -> anyhow::Result<()> {
    let present: Vec<String> = changed
        .iter()
        .filter(|path| top.join(path.as_str()).exists())
        .cloned()
        .collect();
    if !present.is_empty() {
        let mut add_args: Vec<String> = vec!["add".to_string(), "--".to_string()];
        add_args.extend(present);
        git::run(top, &add_args)?;
    }
    let mut commit_args: Vec<String> = vec![
        "commit".to_string(),
        "--quiet".to_string(),
        "--only".to_string(),
        "-m".to_string(),
        message.to_string(),
        "--".to_string(),
    ];
    commit_args.extend(changed.iter().cloned());
    if let Err(err) = git::run(top, &commit_args) {
        let mut reset_args: Vec<String> =
            vec!["reset".to_string(), "-q".to_string(), "--".to_string()];
        reset_args.extend(changed.iter().cloned());
        git::run(top, &reset_args).ok();
        return Err(err);
    }
    Ok(())
}

/// Print what landed: the short sha and one row per committed path.
///
/// # Errors
///
/// Writing `out` failed.
fn print_commit_summary<W: Write>(
    top: &Path,
    changed: &[String],
    out: &mut W,
) -> anyhow::Result<()> {
    let sha = git::run(top, ["rev-parse", "--short", "HEAD"])?;
    writeln!(
        out,
        "Committed {} task file(s) as {}:",
        changed.len(),
        sha.trim()
    )
    .context("printing the commit summary")?;
    for path in changed {
        writeln!(out, "  {path}").context("printing a committed path")?;
    }
    Ok(())
}

/// Each task id's file, repo-relative with `/` separators — the form
/// `git diff --name-only` and `git status` print.
///
/// # Errors
///
/// A task id resolves to nothing, or its file lies outside `top`.
fn resolve_task_paths(store: &Store, top: &Path, ids: &[String]) -> anyhow::Result<Vec<String>> {
    let mut paths: Vec<String> = Vec::new();
    for id in ids {
        let entry = store
            .find(id)?
            .ok_or_else(|| anyhow::anyhow!("task {id} not found"))?;
        let absolute = entry
            .path
            .canonicalize()
            .with_context(|| format!("resolving {}", entry.path.display()))?;
        let relative = absolute.strip_prefix(top).with_context(|| {
            format!(
                "{} is outside the git working tree {}",
                absolute.display(),
                top.display()
            )
        })?;
        let rel = relative
            .components()
            .map(|c| c.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        if !paths.contains(&rel) {
            paths.push(rel);
        }
    }
    for path in deleted_task_paths(store, top, ids)? {
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
    Ok(paths)
}

/// Tracked backlog files that are gone from the working tree (or whose
/// deletion is already staged) and whose `HEAD` content carries one of
/// `ids`: the old path left behind when `task edit -t` renamed a task or
/// `cleanup` moved it to `completed/`. Without them the commit would record
/// the new path but leave `HEAD` holding the old one too — two files with
/// one id.
///
/// # Errors
///
/// The backlog root lies outside `top`, or a git listing fails.
fn deleted_task_paths(store: &Store, top: &Path, ids: &[String]) -> anyhow::Result<Vec<String>> {
    let root = store
        .root()
        .canonicalize()
        .with_context(|| format!("resolving {}", store.root().display()))?;
    let root_rel = root
        .strip_prefix(top)
        .with_context(|| {
            format!(
                "{} is outside the git working tree {}",
                root.display(),
                top.display()
            )
        })?
        .components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/");
    let pathspec = if root_rel.is_empty() {
        ".".to_string()
    } else {
        root_rel
    };
    // Unstaged deletions are still in the index; staged ones are only in
    // `HEAD`. Both are the task's own path.
    let mut candidates = git::nul_records(&git::run(
        top,
        ["ls-files", "--deleted", "-z", "--", pathspec.as_str()],
    )?);
    candidates.extend(git::nul_records(&git::run(
        top,
        [
            "diff",
            "--cached",
            "--name-only",
            "--no-renames",
            "--diff-filter=D",
            "-z",
            "--",
            pathspec.as_str(),
        ],
    )?));
    let wanted: Vec<String> = ids.iter().map(|id| id.to_ascii_lowercase()).collect();
    let mut found: Vec<String> = Vec::new();
    for path in candidates {
        if found.contains(&path)
            || Path::new(&path).extension().and_then(|e| e.to_str()) != Some("md")
        {
            continue;
        }
        // A path with no `HEAD` blob or unparseable content is not a task
        // file of ours; skip it rather than fail the whole commit.
        let Ok(src) = git::run(top, ["show", &format!("HEAD:{path}")]) else {
            continue;
        };
        let Ok(doc) = TaskDoc::parse(&src) else {
            continue;
        };
        if wanted.contains(&doc.frontmatter.id.to_ascii_lowercase()) {
            found.push(path);
        }
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::cmd::git::test_support::{git, init_repo};

    fn task(id: &str) -> String {
        format!(
            "---\nid: {id}\ntitle: 'task {id}'\nstatus: To Do\nassignee: []\n\
             created_date: '2026-01-01 00:00'\nlabels: []\ndependencies: []\n---\n"
        )
    }

    /// A git repo with three committed tasks; returns the dir and the store.
    fn repo() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        init_repo(root);
        let tasks = root.join(".backlog/tasks");
        std::fs::create_dir_all(&tasks).expect("tasks");
        for n in 1..=3 {
            std::fs::write(
                tasks.join(format!("task-{n} - t.md")),
                task(&format!("TASK-{n}")),
            )
            .expect("seed");
        }
        git(root, &["add", "."]);
        git(root, &["commit", "-q", "-m", "seed"]);
        let store = Store::open(&root.join(".backlog")).expect("open");
        (dir, store)
    }

    fn edit(dir: &tempfile::TempDir, n: u32) {
        let path = dir.path().join(format!(".backlog/tasks/task-{n} - t.md"));
        let mut text = std::fs::read_to_string(&path).expect("read");
        text.push_str("\nedited\n");
        std::fs::write(path, text).expect("write");
    }

    fn commit(store: &Store, dir: &tempfile::TempDir, ids: &[&str]) -> anyhow::Result<String> {
        let mut out = Vec::new();
        run_commit(
            store,
            dir.path(),
            &CommitOptions {
                task_ids: ids.iter().map(ToString::to_string).collect(),
                message: "chore(backlog): test".to_string(),
            },
            &mut out,
        )?;
        Ok(String::from_utf8(out).expect("utf8"))
    }

    fn head_files(dir: &tempfile::TempDir) -> Vec<String> {
        git(
            dir.path(),
            &[
                "show",
                "--name-only",
                "--no-renames",
                "--pretty=format:",
                "HEAD",
            ],
        )
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
    }

    /// AC #1: only the listed tasks' changed files land; another task's
    /// unstaged edit stays in the working tree, and an unchanged listed task
    /// contributes nothing.
    #[test]
    fn commits_only_the_listed_changed_task_files() {
        let (dir, store) = repo();
        edit(&dir, 1);
        edit(&dir, 3); // not listed: another wave's edit
        let text = commit(&store, &dir, &["TASK-1", "TASK-2"]).expect("commit");
        assert!(text.contains("Committed 1 task file(s)"), "got: {text}");
        assert_eq!(head_files(&dir), vec![".backlog/tasks/task-1 - t.md"]);
        let status = git(dir.path(), &["status", "--porcelain"]);
        assert!(
            status.contains("task-3 - t.md"),
            "the unlisted edit must stay uncommitted: {status}"
        );
    }

    /// AC #2: a foreign staged path aborts before anything is touched — the
    /// index is exactly as it was and no commit is made.
    #[test]
    fn a_foreign_staged_path_aborts_and_leaves_the_index_alone() {
        let (dir, store) = repo();
        edit(&dir, 1);
        edit(&dir, 3);
        git(dir.path(), &["add", "--", ".backlog/tasks/task-3 - t.md"]);
        let before_index = git(dir.path(), &["diff", "--cached", "--name-only"]);
        let before_head = git(dir.path(), &["rev-parse", "HEAD"]);
        let err = commit(&store, &dir, &["TASK-1"]).expect_err("must refuse");
        assert!(format!("{err:#}").contains("task-3 - t.md"), "got: {err:#}");
        assert_eq!(
            git(dir.path(), &["diff", "--cached", "--name-only"]),
            before_index
        );
        assert_eq!(git(dir.path(), &["rev-parse", "HEAD"]), before_head);
    }

    /// AC #3: no changed task files means no commit and an error that says
    /// so.
    #[test]
    fn no_changes_is_an_error_and_no_commit() {
        let (dir, store) = repo();
        let before_head = git(dir.path(), &["rev-parse", "HEAD"]);
        let err = commit(&store, &dir, &["TASK-1", "TASK-2"]).expect_err("must refuse");
        assert!(
            format!("{err:#}").contains("nothing to commit"),
            "got: {err:#}"
        );
        assert_eq!(git(dir.path(), &["rev-parse", "HEAD"]), before_head);
    }

    /// A listed task file that is already staged is ours, not foreign, and a
    /// brand-new untracked task file is committed too.
    #[test]
    fn own_staged_and_new_task_files_are_committed() {
        let (dir, store) = repo();
        edit(&dir, 1);
        git(dir.path(), &["add", "--", ".backlog/tasks/task-1 - t.md"]);
        std::fs::write(
            dir.path().join(".backlog/tasks/task-4 - t.md"),
            task("TASK-4"),
        )
        .expect("new");
        commit(&store, &dir, &["TASK-1", "TASK-4"]).expect("commit");
        let mut files = head_files(&dir);
        files.sort();
        assert_eq!(
            files,
            vec![
                ".backlog/tasks/task-1 - t.md",
                ".backlog/tasks/task-4 - t.md"
            ]
        );
    }

    /// A renamed task (as `task edit -t` leaves it: old path deleted, new
    /// path untracked) commits both sides, so `HEAD` holds one file per id;
    /// an unlisted task's deletion stays out.
    #[test]
    fn a_renamed_task_commits_the_old_paths_deletion() {
        let (dir, store) = repo();
        let tasks = dir.path().join(".backlog/tasks");
        std::fs::rename(tasks.join("task-1 - t.md"), tasks.join("task-1 - new.md"))
            .expect("rename");
        std::fs::remove_file(tasks.join("task-3 - t.md")).expect("unlisted delete");
        commit(&store, &dir, &["TASK-1"]).expect("commit");
        let mut files = head_files(&dir);
        files.sort();
        assert_eq!(
            files,
            vec![
                ".backlog/tasks/task-1 - new.md",
                ".backlog/tasks/task-1 - t.md"
            ]
        );
        let tree = git(dir.path(), &["ls-tree", "-r", "--name-only", "HEAD"]);
        assert!(!tree.contains("task-1 - t.md"), "old path gone: {tree}");
        assert!(tree.contains("task-3 - t.md"), "unlisted kept: {tree}");
    }

    /// A deletion of the task's old path that is already staged (a
    /// `git mv`) is the task's own path, not a foreign one.
    #[test]
    fn a_staged_rename_is_the_tasks_own_path() {
        let (dir, store) = repo();
        git(
            dir.path(),
            &[
                "mv",
                ".backlog/tasks/task-2 - t.md",
                ".backlog/tasks/task-2 - moved.md",
            ],
        );
        commit(&store, &dir, &["TASK-2"]).expect("commit");
        let tree = git(dir.path(), &["ls-tree", "-r", "--name-only", "HEAD"]);
        assert!(tree.contains("task-2 - moved.md"), "new path: {tree}");
        assert!(!tree.contains("task-2 - t.md"), "old path gone: {tree}");
        assert_eq!(git(dir.path(), &["diff", "--cached", "--name-only"]), "");
    }

    #[test]
    fn unknown_task_id_is_named() {
        let (dir, store) = repo();
        let err = commit(&store, &dir, &["TASK-99"]).expect_err("unknown");
        assert!(format!("{err:#}").contains("TASK-99"), "got: {err:#}");
    }
}
