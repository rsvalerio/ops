//! Shared helpers for Rust-specific data providers.

use std::path::Path;

/// Expand workspace member glob patterns (e.g. `crates/*`) into actual directory paths
/// relative to the workspace root. Non-glob entries are passed through as-is.
pub(crate) fn resolve_member_globs(members: &[String], workspace_root: &Path) -> Vec<String> {
    let mut resolved = Vec::new();
    for member in members {
        if member.contains('*') {
            if let Some(idx) = member.find('*') {
                let prefix = &member[..idx];
                let parent = workspace_root.join(prefix);
                if let Ok(entries) = std::fs::read_dir(&parent) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_dir() && path.join("Cargo.toml").exists() {
                            if let Ok(rel) = path.strip_prefix(workspace_root) {
                                resolved.push(rel.to_string_lossy().to_string());
                            }
                        }
                    }
                }
            }
        } else {
            resolved.push(member.clone());
        }
    }
    resolved.sort();
    resolved
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_member_globs_expands_glob() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        std::fs::create_dir_all(root.join("crates/foo")).unwrap();
        std::fs::write(
            root.join("crates/foo/Cargo.toml"),
            "[package]\nname=\"foo\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("crates/bar")).unwrap();
        std::fs::write(
            root.join("crates/bar/Cargo.toml"),
            "[package]\nname=\"bar\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("crates/not-a-crate")).unwrap();

        let members = vec!["crates/*".to_string()];
        let resolved = resolve_member_globs(&members, root);

        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[0], "crates/bar");
        assert_eq!(resolved[1], "crates/foo");
    }

    #[test]
    fn resolve_member_globs_non_glob_passthrough() {
        let members = vec!["crates/core".to_string(), "crates/cli".to_string()];
        let resolved = resolve_member_globs(&members, std::path::Path::new("/nonexistent"));
        assert_eq!(
            resolved,
            vec!["crates/cli".to_string(), "crates/core".to_string()]
        );
    }

    #[test]
    fn resolve_member_globs_empty_members() {
        let resolved = resolve_member_globs(&[], std::path::Path::new("/nonexistent"));
        assert!(resolved.is_empty());
    }

    #[test]
    fn resolve_member_globs_nonexistent_glob_parent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let resolved = resolve_member_globs(&["crates/*".to_string()], dir.path());
        assert!(resolved.is_empty());
    }
}
