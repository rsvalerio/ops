//! The throwaway review worktree.
//!
//! A detached checkout of HEAD switched to an orphan branch whose only
//! commit is empty, so every file of the crate is untracked and `coderabbit review --uncommitted --include-untracked` reviews
//! the whole of it.
//!
//! [`ReviewWorktree`] owns the worktree, the temp branch and the temp
//! directory holding them; dropping it removes all three, so an error or an
//! interrupted review never leaves them behind in the main checkout.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use ops_backlog::cmd::git;

/// A review worktree and its orphan branch, removed again on drop.
#[derive(Debug)]
pub struct ReviewWorktree {
    /// Top level of the main checkout the worktree was added from.
    top: PathBuf,
    /// The worktree's own directory.
    path: PathBuf,
    /// The orphan branch, once `checkout --orphan` created it.
    branch: Option<String>,
    /// The temp directory holding `path`; deleted after the worktree is
    /// unregistered (field order: dropped after [`Drop::drop`] runs).
    _scratch: tempfile::TempDir,
}

/// Ref- and path-safe form of a target name: ASCII alphanumerics, `.`, `_`
/// and `-` kept, anything else turned into `-`.
pub fn slug(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_start_matches(['.', '-'])
        .to_string()
}

/// The orphan branch name for a target.
pub fn branch_name(target: &str) -> String {
    format!("cr-full-{}", slug(target))
}

impl ReviewWorktree {
    /// Add the worktree and turn it into an empty-base orphan branch: steps
    /// 1 and 2 of the review. Anything created before a failing step is
    /// removed again before the error returns.
    ///
    /// # Errors
    ///
    /// The orphan branch already exists (an earlier run was killed before it
    /// could clean up; the error names the commands that remove it), the
    /// temp directory cannot be created, or a git step fails.
    pub fn create(top: &Path, target: &str) -> anyhow::Result<Self> {
        let branch = branch_name(target);
        if git::branch_exists(top, &branch)? {
            anyhow::bail!(
                "branch {branch} already exists, left by an earlier run that could not clean \
                 up — remove it with `git worktree prune && git branch -D {branch}` (and any \
                 worktree `git worktree list` shows on it) and re-run"
            );
        }
        let scratch = tempfile::Builder::new()
            .prefix("ops-cr-")
            .tempdir()
            .context("creating the review worktree's temp directory")?;
        // Canonical, so an absolute path coderabbit prints (macOS reports
        // `/private/var/…` for `/var/…`) still strips to a repo path.
        let path = scratch
            .path()
            .canonicalize()
            .context("resolving the review worktree's temp directory")?
            .join(format!("ops-cr-{}", slug(target)));
        git::run(
            top,
            [
                OsString::from("worktree"),
                "add".into(),
                "--quiet".into(),
                "--detach".into(),
                path.as_os_str().to_os_string(),
                "HEAD".into(),
            ],
        )
        .with_context(|| format!("adding the review worktree for {target}"))?;
        let mut worktree = Self {
            top: top.to_path_buf(),
            path,
            branch: None,
            _scratch: scratch,
        };
        git::run(&worktree.path, ["checkout", "--quiet", "--orphan", &branch])?;
        worktree.branch = Some(branch);
        // Files stay on disk, untracked; only the index is emptied.
        git::run(&worktree.path, ["rm", "-r", "--cached", "--quiet", "."])?;
        // A fixed identity, no signing and no hooks: this commit never leaves
        // the throwaway branch, and must not fail on a host with no identity
        // or a signing setup that prompts.
        git::run(
            &worktree.path,
            [
                "-c",
                "user.name=ops",
                "-c",
                "user.email=ops@localhost",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "--quiet",
                "--no-verify",
                "--allow-empty",
                "-m",
                "empty base",
            ],
        )?;
        Ok(worktree)
    }

    /// The worktree's directory.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ReviewWorktree {
    fn drop(&mut self) {
        // Best effort, both steps: the caller is already returning (an error
        // or the run's result) and a failed cleanup must not mask it. Each
        // failure is surfaced with the command that finishes the job by hand.
        if let Err(err) = git::run(
            &self.top,
            [
                OsString::from("worktree"),
                "remove".into(),
                "--force".into(),
                self.path.as_os_str().to_os_string(),
            ],
        ) {
            tracing::warn!(
                path = ?self.path.display(),
                error = %err,
                "failed to remove the review worktree; run `git worktree prune` once the \
                 directory is gone"
            );
        }
        if let Some(branch) = &self.branch
            && let Err(err) = git::run(&self.top, ["branch", "-D", branch])
        {
            tracing::warn!(
                branch = ?branch,
                error = %err,
                "failed to delete the review branch; delete it with `git branch -D`"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn git_ok(dir: &Path, args: &[&str]) -> String {
        let out = Command::new("git")
            .arg("-C")
            .arg(dir)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE")
            .args(args)
            .output()
            .expect("spawn git");
        assert!(
            out.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    fn repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        git_ok(root, &["init", "-q", "-b", "main"]);
        git_ok(root, &["config", "user.email", "t@example.com"]);
        git_ok(root, &["config", "user.name", "t"]);
        git_ok(root, &["config", "commit.gpgsign", "false"]);
        std::fs::create_dir_all(root.join("crates/a/src")).expect("mkdir");
        std::fs::write(root.join("crates/a/src/lib.rs"), "fn a() {}\n").expect("write");
        git_ok(root, &["add", "."]);
        git_ok(root, &["commit", "-q", "-m", "init"]);
        dir
    }

    #[test]
    fn slug_keeps_ref_safe_characters_only() {
        assert_eq!(slug("ops-core"), "ops-core");
        assert_eq!(slug("a b/c~d"), "a-b-c-d");
        assert_eq!(slug("..x"), "x");
        assert_eq!(branch_name("ops core"), "cr-full-ops-core");
    }

    #[test]
    fn worktree_has_every_file_untracked_on_an_empty_orphan_and_is_removed_on_drop() {
        let dir = repo();
        let top = dir.path().canonicalize().expect("canon");
        let head_before = git_ok(&top, &["rev-parse", "HEAD"]);
        let path = {
            let wt = ReviewWorktree::create(&top, "a").expect("create");
            let path = wt.path().to_path_buf();
            assert!(path.join("crates/a/src/lib.rs").is_file());
            let status = git_ok(&path, &["status", "--porcelain", "--untracked-files=all"]);
            assert_eq!(status, "?? crates/a/src/lib.rs\n");
            assert_eq!(git_ok(&path, &["rev-list", "--count", "HEAD"]).trim(), "1");
            assert_eq!(
                git_ok(&path, &["ls-tree", "-r", "--name-only", "HEAD"]),
                "",
                "the base commit is empty"
            );
            path
        };
        assert!(!path.exists(), "worktree directory removed");
        assert!(!git::branch_exists(&top, "cr-full-a").expect("show-ref"));
        assert!(!git_ok(&top, &["worktree", "list"]).contains("ops-cr-a"));
        assert_eq!(git_ok(&top, &["rev-parse", "HEAD"]), head_before);
        assert_eq!(git_ok(&top, &["status", "--porcelain"]), "");
        assert_eq!(git_ok(&top, &["branch", "--show-current"]).trim(), "main");
    }

    #[test]
    fn a_leftover_branch_is_refused_before_anything_is_created() {
        let dir = repo();
        let top = dir.path().canonicalize().expect("canon");
        git_ok(&top, &["branch", "cr-full-a"]);
        let err = ReviewWorktree::create(&top, "a").expect_err("must refuse");
        assert!(err.to_string().contains("git branch -D cr-full-a"), "{err}");
        assert_eq!(git_ok(&top, &["worktree", "list"]).lines().count(), 1);
    }
}
