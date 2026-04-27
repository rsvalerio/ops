//! Shared helpers for Rust-specific data providers.
//!
//! ## Why not `MetadataProvider`?
//!
//! `cargo metadata` is the canonical source for resolved workspace members,
//! but invoking it requires running `cargo` (slow, network-touching, and
//! requires a fully-resolvable lockfile). The about/identity/units/coverage
//! providers run on every `ops about` invocation and need to be cheap and
//! offline-tolerant. They therefore parse `Cargo.toml` directly and resolve
//! workspace globs with the static expander below. `MetadataProvider` is used
//! by `deps_provider` where dependency graph data is unavoidable.

use ops_cargo_toml::{CargoToml, CargoTomlProvider};
use ops_extension::{Context, DataProvider, DataProviderError};
use std::path::Path;

/// Load and parse `Cargo.toml` for the current context, then resolve any
/// `[workspace].members` globs in place. Reuses any value already cached at
/// the `cargo_toml` key; otherwise reads via [`CargoTomlProvider`].
/// Centralises the parse + glob-resolve step that identity / units /
/// coverage providers all need (TASK-0381).
pub(crate) fn load_workspace_manifest(ctx: &mut Context) -> Result<CargoToml, DataProviderError> {
    let value = if let Some(cached) = ctx.cached(ops_cargo_toml::DATA_PROVIDER_NAME) {
        (**cached).clone()
    } else {
        CargoTomlProvider::new().provide(ctx)?
    };
    let mut manifest: CargoToml =
        serde_json::from_value(value).map_err(DataProviderError::computation_error)?;

    let cwd = ctx.working_directory.clone();
    let resolved = resolved_workspace_members(&manifest, &cwd);
    if let Some(ws) = manifest.workspace.as_mut() {
        ws.members = resolved;
    }

    Ok(manifest)
}

/// Resolve `[workspace].members` globs to concrete member paths, honoring
/// `[workspace].exclude`. Members without a `*` are passed through verbatim.
///
/// Supports the simple `prefix/*` shape Cargo workspaces use in practice.
/// More elaborate patterns (`prefix/*/suffix`, `**`, `?`, character classes)
/// are not expanded — they are passed through unchanged and a `tracing::warn!`
/// is emitted so the unsupported shape is visible in logs rather than silently
/// producing a wrong member list.
pub(crate) fn resolved_workspace_members(
    manifest: &CargoToml,
    workspace_root: &Path,
) -> Vec<String> {
    let Some(ws) = manifest.workspace.as_ref() else {
        return Vec::new();
    };

    let exclude: std::collections::HashSet<&str> = ws.exclude.iter().map(String::as_str).collect();

    let mut resolved = Vec::new();
    for member in &ws.members {
        if let Some(idx) = member.find('*') {
            if is_unsupported_glob(member, idx) {
                tracing::warn!(
                    pattern = %member,
                    "workspace member glob shape not supported by ops about; passing through unchanged"
                );
                resolved.push(member.clone());
                continue;
            }
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
        } else {
            resolved.push(member.clone());
        }
    }

    resolved.retain(|m| !exclude.contains(m.as_str()));
    resolved.sort();
    resolved
}

/// Returns true if the glob shape goes beyond a single trailing `*` after
/// the prefix — anything we cannot expand correctly with the simple
/// `read_dir(prefix)` approach.
fn is_unsupported_glob(member: &str, first_star: usize) -> bool {
    let after_star = &member[first_star + 1..];
    if !after_star.is_empty() {
        return true;
    }
    member.contains('?') || member.contains('[')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest_with_members(members: &[&str]) -> CargoToml {
        let toml_str = format!(
            "[workspace]\nmembers = [{}]\n",
            members
                .iter()
                .map(|m| format!("\"{m}\""))
                .collect::<Vec<_>>()
                .join(", ")
        );
        toml::from_str(&toml_str).expect("parse manifest")
    }

    fn manifest_with_members_and_exclude(members: &[&str], exclude: &[&str]) -> CargoToml {
        let toml_str = format!(
            "[workspace]\nmembers = [{}]\nexclude = [{}]\n",
            members
                .iter()
                .map(|m| format!("\"{m}\""))
                .collect::<Vec<_>>()
                .join(", "),
            exclude
                .iter()
                .map(|m| format!("\"{m}\""))
                .collect::<Vec<_>>()
                .join(", ")
        );
        toml::from_str(&toml_str).expect("parse manifest")
    }

    #[test]
    fn resolves_simple_glob() {
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

        let manifest = manifest_with_members(&["crates/*"]);
        let resolved = resolved_workspace_members(&manifest, root);

        assert_eq!(
            resolved,
            vec!["crates/bar".to_string(), "crates/foo".to_string()]
        );
    }

    #[test]
    fn passthrough_non_glob_members() {
        let manifest = manifest_with_members(&["crates/core", "crates/cli"]);
        let resolved = resolved_workspace_members(&manifest, std::path::Path::new("/nonexistent"));
        assert_eq!(
            resolved,
            vec!["crates/cli".to_string(), "crates/core".to_string()]
        );
    }

    #[test]
    fn empty_when_no_workspace() {
        let manifest: CargoToml =
            toml::from_str("[package]\nname=\"x\"\nversion=\"0.1.0\"\n").expect("parse");
        let resolved = resolved_workspace_members(&manifest, std::path::Path::new("/nonexistent"));
        assert!(resolved.is_empty());
    }

    #[test]
    fn nonexistent_glob_parent_yields_empty() {
        let dir = tempfile::tempdir().expect("tempdir");
        let manifest = manifest_with_members(&["crates/*"]);
        let resolved = resolved_workspace_members(&manifest, dir.path());
        assert!(resolved.is_empty());
    }

    #[test]
    fn exclude_filters_explicit_members() {
        let manifest = manifest_with_members_and_exclude(
            &["crates/core", "crates/experimental"],
            &["crates/experimental"],
        );
        let resolved = resolved_workspace_members(&manifest, std::path::Path::new("/nonexistent"));
        assert_eq!(resolved, vec!["crates/core".to_string()]);
    }

    #[test]
    fn exclude_filters_glob_results() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        for name in ["foo", "bar", "experimental"] {
            std::fs::create_dir_all(root.join(format!("crates/{name}"))).unwrap();
            std::fs::write(
                root.join(format!("crates/{name}/Cargo.toml")),
                "[package]\nname=\"x\"\n",
            )
            .unwrap();
        }

        let manifest = manifest_with_members_and_exclude(&["crates/*"], &["crates/experimental"]);
        let resolved = resolved_workspace_members(&manifest, root);
        assert_eq!(
            resolved,
            vec!["crates/bar".to_string(), "crates/foo".to_string()]
        );
    }

    /// Suffix-after-`*` (e.g. `crates/*/sub`) is not supported by the simple
    /// expander. The pattern is passed through unchanged with a warn-log
    /// rather than silently producing a wrong member list (TASK-0410).
    #[test]
    fn unsupported_suffix_after_star_passes_through() {
        let manifest = manifest_with_members(&["crates/*/sub"]);
        let resolved = resolved_workspace_members(&manifest, std::path::Path::new("/nonexistent"));
        assert_eq!(resolved, vec!["crates/*/sub".to_string()]);
    }

    #[test]
    fn unsupported_globstar_passes_through() {
        let manifest = manifest_with_members(&["crates/**"]);
        let resolved = resolved_workspace_members(&manifest, std::path::Path::new("/nonexistent"));
        assert_eq!(resolved, vec!["crates/**".to_string()]);
    }
}
