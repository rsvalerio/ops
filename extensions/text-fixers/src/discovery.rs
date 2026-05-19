//! Enumerate candidate files for the text fixers.
//!
//! Two modes:
//! - `walk`: recursive directory walk under `root`, skipping a conservative
//!   deny-list of VCS/build directories.
//! - `tracked`: `git ls-files -z` from `root`; falls back to the walk if
//!   git is not available or the directory is not a repo.

use std::path::{Path, PathBuf};
use std::process::Command;

const SKIP_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "dist",
    "build",
    ".venv",
    "venv",
    "__pycache__",
    ".terraform",
];

pub fn discover(root: &Path, tracked_only: bool) -> std::io::Result<Vec<PathBuf>> {
    if tracked_only {
        if let Some(paths) = tracked_files(root) {
            return Ok(paths);
        }
    }
    walk(root)
}

fn walk(root: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let file_type = match entry.file_type() {
                Ok(t) => t,
                Err(_) => continue,
            };
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if SKIP_DIRS.iter().any(|d| *d == name_str) {
                    continue;
                }
                stack.push(path);
            } else if file_type.is_file() {
                out.push(path);
            }
        }
    }
    Ok(out)
}

fn tracked_files(root: &Path) -> Option<Vec<PathBuf>> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-z"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let mut out = Vec::new();
    for chunk in output.stdout.split(|b| *b == 0) {
        if chunk.is_empty() {
            continue;
        }
        let rel = match std::str::from_utf8(chunk) {
            Ok(s) => s,
            Err(_) => continue,
        };
        out.push(root.join(rel));
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walk_skips_deny_listed_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("keep.txt"), b"a").unwrap();
        std::fs::create_dir_all(root.join("target/sub")).unwrap();
        std::fs::write(root.join("target/sub/skip.txt"), b"a").unwrap();
        std::fs::create_dir_all(root.join("node_modules")).unwrap();
        std::fs::write(root.join("node_modules/skip.txt"), b"a").unwrap();

        let files = walk(root).unwrap();
        let names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(names.contains(&"keep.txt".to_string()));
        assert!(!names.iter().any(|n| n == "skip.txt"));
    }

    #[test]
    fn walk_descends_into_normal_subdirs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src/nested")).unwrap();
        std::fs::write(root.join("src/nested/deep.txt"), b"a").unwrap();
        let files = walk(root).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("src/nested/deep.txt"));
    }
}
