//! Git directory discovery.
//!
//! Walks up from a starting path to locate a repo's `.git` directory,
//! resolving worktree pointer files and rejecting symlinked `.git` entries
//! (supply-chain risk for callers that write into the returned path).

use std::path::{Component, Path, PathBuf};

/// Maximum number of parent directories to walk while searching for `.git`.
/// Bounds the loop so a hostile cwd cannot force us to ascend to `/` repeatedly.
const FIND_GIT_DIR_MAX_DEPTH: usize = 64;

/// SEC-14: maximum net `..` traversal allowed in a relative `gitdir:` pointer.
///
/// Real worktree pointers either use absolute paths or step up at most one or
/// two directories to reach the parent repo's `.git/worktrees/<name>`.
/// A pointer with deeper `..` traversal (e.g. `../../../../../etc`) is the
/// shape of a redirection attack against the hook installer, which writes
/// into the resolved path.
const MAX_GITDIR_PARENT_TRAVERSAL: usize = 2;

/// Find the `.git` directory by walking up from the given path.
///
/// Handles three cases:
/// 1. Plain repos: `.git` is a real directory (symlinked `.git` is rejected).
/// 2. Worktrees / submodules: `.git` is a regular file with body
///    `gitdir: <path>`. The path is resolved relative to the working copy root
///    and returned.
/// 3. Otherwise walks up to the parent, up to [`FIND_GIT_DIR_MAX_DEPTH`] times.
///
/// Symlinked `.git` entries are deliberately skipped: callers like the hook
/// installer write into this directory and a redirected symlink is a
/// supply-chain risk. The returned path is canonicalised so downstream
/// consumers see a stable, real location.
///
/// There is no caller-supplied root ceiling — the depth limit serves as the
/// bound. Pass an already-canonicalised input if the caller has a stricter
/// containment requirement.
pub fn find_git_dir(from: &Path) -> Option<PathBuf> {
    let mut dir = from.to_path_buf();
    for _ in 0..FIND_GIT_DIR_MAX_DEPTH {
        if let Some(found) = probe_git_entry(&dir.join(".git")) {
            return Some(found);
        }
        if !dir.pop() {
            return None;
        }
    }
    None
}

fn probe_git_entry(candidate: &Path) -> Option<PathBuf> {
    let meta = std::fs::symlink_metadata(candidate).ok()?;
    let ft = meta.file_type();
    // Symlinked .git is skipped silently — never trust it for writes.
    if ft.is_dir() {
        Some(std::fs::canonicalize(candidate).unwrap_or_else(|_| candidate.to_path_buf()))
    } else if ft.is_file() {
        let resolved = read_gitdir_pointer(candidate)?;
        Some(std::fs::canonicalize(&resolved).unwrap_or(resolved))
    } else {
        None
    }
}

fn read_gitdir_pointer(file: &Path) -> Option<PathBuf> {
    let content = std::fs::read_to_string(file).ok()?;
    let rest = content.lines().find_map(|l| l.strip_prefix("gitdir:"))?;
    let target = Path::new(rest.trim());
    if target.is_absolute() {
        return Some(target.to_path_buf());
    }
    if max_parent_escape(target) > MAX_GITDIR_PARENT_TRAVERSAL {
        return None;
    }
    Some(file.parent()?.join(target))
}

/// SEC-14: peak number of directories `path` ascends above its starting point
/// while being walked component-by-component. `a/../../b` peaks at 1 above
/// start, `../../etc` peaks at 2.
fn max_parent_escape(path: &Path) -> usize {
    let mut depth: i64 = 0;
    let mut peak: i64 = 0;
    for c in path.components() {
        match c {
            Component::ParentDir => {
                depth -= 1;
                if -depth > peak {
                    peak = -depth;
                }
            }
            Component::Normal(_) => depth += 1,
            Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
        }
    }
    usize::try_from(peak).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_git_dir_in_current() {
        let dir = tempfile::tempdir().expect("tempdir");
        let git = dir.path().join(".git");
        std::fs::create_dir(&git).unwrap();
        let expected = std::fs::canonicalize(&git).unwrap();
        assert_eq!(find_git_dir(dir.path()), Some(expected));
    }

    #[test]
    fn find_git_dir_in_parent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let git = dir.path().join(".git");
        std::fs::create_dir(&git).unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        let expected = std::fs::canonicalize(&git).unwrap();
        assert_eq!(find_git_dir(&sub), Some(expected));
    }

    #[test]
    fn find_git_dir_not_found() {
        let dir = tempfile::tempdir().expect("tempdir");
        let result = find_git_dir(dir.path());
        assert!(result.is_none());
    }

    #[test]
    fn find_git_dir_resolves_worktree_pointer_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let real_gitdir = dir.path().join("worktrees/feature");
        std::fs::create_dir_all(&real_gitdir).unwrap();
        let worktree = dir.path().join("checkout");
        std::fs::create_dir(&worktree).unwrap();
        let pointer = worktree.join(".git");
        std::fs::write(&pointer, format!("gitdir: {}\n", real_gitdir.display())).unwrap();
        let expected = std::fs::canonicalize(&real_gitdir).unwrap();
        assert_eq!(find_git_dir(&worktree), Some(expected));
    }

    #[cfg(unix)]
    #[test]
    fn find_git_dir_skips_symlinked_dot_git() {
        let dir = tempfile::tempdir().expect("tempdir");
        let outside = dir.path().join("attacker_repo");
        std::fs::create_dir(&outside).unwrap();
        let workspace = dir.path().join("workspace");
        std::fs::create_dir(&workspace).unwrap();
        std::os::unix::fs::symlink(&outside, workspace.join(".git")).unwrap();
        // Symlinked .git is skipped; with no real .git anywhere, the walk fails.
        assert_eq!(find_git_dir(&workspace), None);
    }

    /// SEC-14: a relative `gitdir:` pointer that traverses several parents to
    /// land on something like `/etc/passwd` must be rejected, even if the
    /// attacker plants a HEAD file in the resolved target so `looks_like_git_dir`
    /// would otherwise accept it.
    #[test]
    fn find_git_dir_rejects_excessive_parent_traversal_in_pointer() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Build a deep-enough chain so `../../../<target>` actually resolves
        // to a real planted directory inside the tempdir.
        let chain = dir.path().join("a/b/c");
        std::fs::create_dir_all(&chain).unwrap();
        let target = dir.path().join("etc_passwd");
        std::fs::create_dir(&target).unwrap();
        // Plant a HEAD file so a downstream looks_like_git_dir check would
        // otherwise accept the redirected target.
        std::fs::write(target.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        let pointer = chain.join(".git");
        std::fs::write(&pointer, "gitdir: ../../../etc_passwd\n").unwrap();

        // No real .git anywhere in the ancestor chain — only the planted
        // pointer. With the SEC-14 traversal bound the pointer is refused
        // and the walk falls through to None.
        assert_eq!(find_git_dir(&chain), None);
    }

    #[test]
    fn max_parent_escape_counts_peak_traversal() {
        assert_eq!(max_parent_escape(Path::new("../actual")), 1);
        assert_eq!(max_parent_escape(Path::new("../../../etc")), 3);
        // Net 1 step up but peak is 2.
        assert_eq!(max_parent_escape(Path::new("../../foo/bar")), 2);
        // No escape — `a/..` cancels out.
        assert_eq!(max_parent_escape(Path::new("a/../b")), 0);
    }

    #[test]
    fn find_git_dir_relative_pointer() {
        let dir = tempfile::tempdir().expect("tempdir");
        let worktree = dir.path().join("checkout");
        std::fs::create_dir_all(worktree.join("../actual_gitdir")).unwrap();
        let pointer = worktree.join(".git");
        std::fs::write(&pointer, "gitdir: ../actual_gitdir\n").unwrap();
        let result = find_git_dir(&worktree).expect("should resolve");
        assert!(result.ends_with("actual_gitdir"));
    }
}
