//! Git directory discovery.
//!
//! Walks up from a starting path to locate a repo's `.git` directory,
//! resolving worktree pointer files and rejecting symlinked `.git` entries
//! (supply-chain risk for callers that write into the returned path).

use std::path::{Path, PathBuf};

/// Maximum number of parent directories to walk while searching for `.git`.
/// Bounds the loop so a hostile cwd cannot force us to ascend to `/` repeatedly.
const FIND_GIT_DIR_MAX_DEPTH: usize = 64;

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
    let resolved = if target.is_absolute() {
        target.to_path_buf()
    } else {
        file.parent()?.join(target)
    };
    Some(resolved)
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
