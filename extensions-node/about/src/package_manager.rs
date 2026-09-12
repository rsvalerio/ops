//! Detect the active Node package manager from a `packageManager` field
//! and/or lockfile presence.
//!
//! Unknown `packageManager` values (labels we don't enumerate, e.g. `"deno"`
//! or a typo like `"pnmp"`) are treated as unset, mirroring the
//! whitespace-only branch: control falls through to the lockfile probe so a
//! real `pnpm-lock.yaml` / `yarn.lock` / etc. still surfaces in the About
//! card. A `tracing::debug!` breadcrumb records the unrecognised label.

use std::path::Path;

/// Resolve the package-manager label for the About card from
/// `package.json`'s `packageManager` value and the lockfiles present in
/// `project_root`, or `None` when neither identifies one.
pub fn detect_package_manager(
    project_root: &Path,
    has_packagemanager: Option<&str>,
) -> Option<&'static str> {
    // The `packageManager` field takes precedence, but an empty or
    // whitespace-only value is a real corepack-disable pattern and an
    // unrecognised label carries no usable hint. Both are treated as unset so
    // control reaches the lockfile probe, rather than returning `None` and
    // hiding a lockfile that is actually there.
    if let Some(pm) = has_packagemanager.map(str::trim).filter(|s| !s.is_empty()) {
        let name = pm.split_once('@').map_or(pm, |(n, _)| n);
        match name {
            "pnpm" => return Some("pnpm"),
            "yarn" => return Some("yarn"),
            "npm" => return Some("npm"),
            "bun" => return Some("bun"),
            other => {
                tracing::debug!(
                    unrecognised = other,
                    "unrecognised packageManager label; falling through to lockfile probe"
                );
            }
        }
    }
    // A pure presence probe: the result is a static label (`"pnpm"`,
    // `"yarn"`, …) and the lockfile contents are never read, so there is no
    // check-then-read race to collapse. `symlink_metadata` rather than
    // `exists()` keeps a symlinked lockfile from being followed to an
    // arbitrary target, and costs one syscall fewer.
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

    #[test]
    fn unknown_package_manager_falls_through_to_lockfile_probe() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();
        assert_eq!(
            detect_package_manager(dir.path(), Some("deno@2.0.0")),
            Some("pnpm")
        );
        assert_eq!(
            detect_package_manager(dir.path(), Some("pnmp@8.6.1")),
            Some("pnpm")
        );
    }

    #[test]
    fn unknown_package_manager_without_lockfile_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(detect_package_manager(dir.path(), Some("deno")), None);
        assert_eq!(detect_package_manager(dir.path(), Some("yarn-pnp@4")), None);
    }

    #[test]
    fn known_package_manager_with_version_still_recognised() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            detect_package_manager(dir.path(), Some("pnpm@8.6.1")),
            Some("pnpm")
        );
    }

    #[test]
    fn empty_package_manager_field_falls_through_to_lockfile_probe() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();
        assert_eq!(detect_package_manager(dir.path(), Some("")), Some("pnpm"));
        assert_eq!(
            detect_package_manager(dir.path(), Some("   ")),
            Some("pnpm")
        );
    }
}
