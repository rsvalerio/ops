//! Path canonicalization and validation for hook installation.
//!
//! Centralises the symlink-defense and `.git`-shape checks so security-relevant
//! invariants live in one place rather than mixed with write logic in
//! `install.rs`.

use std::path::{Path, PathBuf};

use anyhow::Context;

pub(crate) fn canonical_git_dir(git_dir: &Path) -> anyhow::Result<PathBuf> {
    let canonical = std::fs::canonicalize(git_dir)
        .with_context(|| format!("failed to canonicalize git_dir {}", git_dir.display()))?;
    if !is_accepted_git_dir(&canonical) {
        anyhow::bail!(
            "refusing to install hook: {} is not a .git directory or worktree gitdir",
            canonical.display()
        );
    }
    Ok(canonical)
}

pub(crate) fn canonical_subdir(parent: &Path, child: &Path) -> anyhow::Result<PathBuf> {
    let canonical = std::fs::canonicalize(child)
        .with_context(|| format!("failed to canonicalize {}", child.display()))?;
    let symlink_meta = std::fs::symlink_metadata(child)
        .with_context(|| format!("failed to stat {}", child.display()))?;
    if symlink_meta.file_type().is_symlink() {
        anyhow::bail!("refusing to install hook: {} is a symlink", child.display());
    }
    if !canonical.starts_with(parent) {
        anyhow::bail!(
            "refusing to install hook: {} resolves outside {}",
            canonical.display(),
            parent.display()
        );
    }
    Ok(canonical)
}

fn is_accepted_git_dir(path: &Path) -> bool {
    has_accepted_filename(path) && looks_like_git_dir(path)
}

fn has_accepted_filename(path: &Path) -> bool {
    if path.file_name().is_some_and(|n| n == ".git") {
        return true;
    }
    // Worktree gitdir: `<repo>/.git/worktrees/<name>`.
    let parent = path.parent();
    let grandparent = parent.and_then(Path::parent);
    parent.is_some_and(|p| p.file_name().is_some_and(|n| n == "worktrees"))
        && grandparent.is_some_and(|g| g.file_name().is_some_and(|n| n == ".git"))
}

/// Substance check: a real git dir always has a `HEAD` file. We require it as
/// a sanity check so a directory merely *named* `.git` (e.g. an attacker-
/// controlled `/tmp/.git`) is not accepted by the filename heuristic alone.
fn looks_like_git_dir(path: &Path) -> bool {
    path.join("HEAD").is_file()
}
