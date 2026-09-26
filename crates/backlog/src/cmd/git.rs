//! The one place the backlog commands shell out to `git`.
//!
//! Every call is `git -C <dir> <args…>` through [`std::process::Command`] —
//! never a shell — so task paths, branch names and commit messages reach git
//! as single argv entries and cannot be reinterpreted (SEC-13). Callers pass
//! `--` before any path list so a path starting with `-` is never read as an
//! option.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Context as _;

/// Run `git -C <dir> <args…>` and return its stdout.
///
/// # Errors
///
/// `git` cannot be spawned, or it exits non-zero — the error carries the
/// subcommand and git's own stderr, trimmed.
pub fn run<I, S>(dir: &Path, args: I) -> anyhow::Result<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let args: Vec<std::ffi::OsString> = args
        .into_iter()
        .map(|a| a.as_ref().to_os_string())
        .collect();
    let subcommand = args
        .first()
        .map_or_else(String::new, |a| a.to_string_lossy().into_owned());
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        // Task file names are data, not globs: a `*` or `[` in a slug must
        // match only itself.
        .env("GIT_LITERAL_PATHSPECS", "1")
        // `-C <dir>` must decide the repository: a GIT_DIR / GIT_WORK_TREE /
        // GIT_INDEX_FILE inherited from an enclosing git hook would silently
        // redirect every step to another repository's index.
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .args(&args)
        .output()
        .with_context(|| format!("running git {subcommand} in {}", dir.display()))?;
    if !output.status.success() {
        anyhow::bail!(
            "git {subcommand} failed in {}: {}",
            dir.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// The working tree's top-level directory containing `dir`, canonicalized.
///
/// # Errors
///
/// `dir` is not inside a git working tree, or the path cannot be
/// canonicalized.
pub fn toplevel(dir: &Path) -> anyhow::Result<PathBuf> {
    let raw = run(dir, ["rev-parse", "--show-toplevel"])?;
    let top = PathBuf::from(raw.trim_end_matches(['\n', '\r']));
    top.canonicalize()
        .with_context(|| format!("resolving {}", top.display()))
}

/// Split `-z` output (NUL-terminated records) into owned strings.
pub fn nul_records(raw: &str) -> Vec<String> {
    raw.split('\0')
        .filter(|record| !record.is_empty())
        .map(str::to_string)
        .collect()
}

/// The paths currently staged in the index, repo-relative.
///
/// # Errors
///
/// As [`run`].
pub fn staged_paths(dir: &Path) -> anyhow::Result<Vec<String>> {
    Ok(nul_records(&run(
        dir,
        ["diff", "--cached", "--name-only", "-z"],
    )?))
}

/// Does the local branch `name` exist?
///
/// # Errors
///
/// `git` cannot be spawned.
pub fn branch_exists(dir: &Path, name: &str) -> anyhow::Result<bool> {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .args(["show-ref", "--verify", "--quiet"])
        .arg(format!("refs/heads/{name}"))
        .status()
        .with_context(|| format!("running git show-ref in {}", dir.display()))?;
    Ok(status.success())
}

/// Scratch-repository helpers shared by the git-backed command tests.
#[cfg(test)]
pub mod test_support {
    use std::path::Path;
    use std::process::Command;

    /// Run `git -C <dir> <args…>` with any hook-inherited repository
    /// environment removed, asserting success; returns stdout.
    pub fn git(dir: &Path, args: &[&str]) -> String {
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

    /// `git init` on `main` with a local identity and signing off, so
    /// commits work on any machine.
    pub fn init_repo(root: &Path) {
        git(root, &["init", "-q", "-b", "main"]);
        git(root, &["config", "user.email", "t@example.com"]);
        git(root, &["config", "user.name", "t"]);
        git(root, &["config", "commit.gpgsign", "false"]);
    }
}
