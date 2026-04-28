//! Detect the active Node package manager from a `packageManager` field
//! and/or lockfile presence.

use std::path::Path;

pub(crate) fn detect_package_manager(
    project_root: &Path,
    has_packagemanager: Option<&str>,
) -> Option<&'static str> {
    // `packageManager` field takes precedence.
    if let Some(pm) = has_packagemanager {
        let name = pm.split_once('@').map_or(pm, |(n, _)| n);
        return match name {
            "pnpm" => Some("pnpm"),
            "yarn" => Some("yarn"),
            "npm" => Some("npm"),
            "bun" => Some("bun"),
            _ => None,
        };
    }
    // SEC-25 / TASK-0392: this branch is a pure presence probe — the result is
    // a static label (`"pnpm"`, `"yarn"`, ...), and the lockfile contents are
    // never read afterwards. There is no follow-up `read_to_string` to merge
    // with, so leaving the probe as a metadata stat is acceptable. Using
    // `symlink_metadata` (rather than `exists()`) avoids following a symlinked
    // lockfile to an arbitrary target and removes one syscall round-trip.
    if probe(project_root, "pnpm-lock.yaml") || probe(project_root, "pnpm-workspace.yaml") {
        return Some("pnpm");
    }
    if probe(project_root, "yarn.lock") {
        return Some("yarn");
    }
    if probe(project_root, "bun.lockb") || probe(project_root, "bun.lock") {
        return Some("bun");
    }
    if probe(project_root, "package-lock.json") {
        return Some("npm");
    }
    None
}

fn probe(dir: &Path, name: &str) -> bool {
    std::fs::symlink_metadata(dir.join(name)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_manager_field_recognizes_bun() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            detect_package_manager(dir.path(), Some("bun@1.1.0")),
            Some("bun")
        );
    }

    #[test]
    fn package_manager_field_without_version() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            detect_package_manager(dir.path(), Some("pnpm")),
            Some("pnpm")
        );
        assert_eq!(detect_package_manager(dir.path(), Some("bun")), Some("bun"));
    }
}
