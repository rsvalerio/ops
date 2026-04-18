//! Rust-specific `project_units` data provider.
//!
//! Reads `[workspace].members` from Cargo.toml and per-crate Cargo manifests
//! for display metadata. LOC/file counts are enriched by the generic
//! `run_about_units` runner when DuckDB is available.

use ops_about::cards::format_unit_name;
use ops_core::project_identity::ProjectUnit;
use ops_extension::{Context, DataProvider, DataProviderError};

use crate::query::resolve_member_globs;
use ops_cargo_toml::CargoToml;

pub(crate) const PROVIDER_NAME: &str = "project_units";

pub(crate) struct RustUnitsProvider;

impl DataProvider for RustUnitsProvider {
    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let cwd = ctx.working_directory.clone();

        let cargo_value = match ctx.data_cache.get("cargo_toml") {
            Some(v) => v.clone(),
            None => {
                // No cached cargo_toml — try to read directly from filesystem.
                return Ok(serde_json::to_value(Vec::<ProjectUnit>::new())?);
            }
        };
        let mut manifest: CargoToml = serde_json::from_value((*cargo_value).clone())?;

        if let Some(ws) = &mut manifest.workspace {
            ws.members = resolve_member_globs(&ws.members, &cwd);
        }

        let members = match &manifest.workspace {
            Some(ws) if !ws.members.is_empty() => ws.members.clone(),
            _ => Vec::new(),
        };

        // Per-crate dep counts from DuckDB (Rust-specific, keyed by package name).
        let dep_counts: std::collections::HashMap<String, i64> = ops_duckdb::get_db(ctx)
            .and_then(|db| ops_duckdb::sql::query_crate_dep_counts(db).ok())
            .unwrap_or_default();

        let mut sorted_members = members;
        sorted_members.sort();

        let units: Vec<ProjectUnit> = sorted_members
            .iter()
            .map(|member| {
                let crate_toml = cwd.join(member).join("Cargo.toml");
                let (pkg_name, version, description) = read_crate_metadata(&crate_toml);
                let package_name = pkg_name.unwrap_or_default();
                let dep_count = dep_counts.get(&package_name).copied();

                ProjectUnit {
                    name: format_unit_name(member),
                    path: member.clone(),
                    version,
                    description,
                    loc: None,
                    file_count: None,
                    dep_count,
                }
            })
            .collect();

        serde_json::to_value(&units).map_err(DataProviderError::from)
    }
}

/// Read package name, version, and description from a crate's Cargo.toml.
pub(crate) fn read_crate_metadata(
    crate_toml_path: &std::path::Path,
) -> (Option<String>, Option<String>, Option<String>) {
    let content = match std::fs::read_to_string(crate_toml_path) {
        Ok(c) => c,
        Err(_) => return (None, None, None),
    };

    let parsed: toml::Value = match toml::from_str(&content) {
        Ok(p) => p,
        Err(_) => return (None, None, None),
    };

    let package = parsed.get("package");
    let name = package
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(|s| s.to_string());
    let version = package
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let description = package
        .and_then(|p| p.get("description"))
        .and_then(|d| d.as_str())
        .map(|s| s.to_string());

    (name, version, description)
}

/// Resolve display name for a member by reading its Cargo.toml, falling back
/// to the capitalized last path segment.
pub(crate) fn resolve_crate_display_name(member: &str, workspace_root: &std::path::Path) -> String {
    let toml_path = workspace_root.join(member).join("Cargo.toml");
    let (pkg_name, _, _) = read_crate_metadata(&toml_path);
    pkg_name.unwrap_or_else(|| format_unit_name(member))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_crate_metadata_basic() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Cargo.toml");
        std::fs::write(
            &path,
            "[package]\nname = \"foo\"\nversion = \"1.0.0\"\ndescription = \"a foo\"\n",
        )
        .unwrap();
        let (name, version, desc) = read_crate_metadata(&path);
        assert_eq!(name.as_deref(), Some("foo"));
        assert_eq!(version.as_deref(), Some("1.0.0"));
        assert_eq!(desc.as_deref(), Some("a foo"));
    }

    #[test]
    fn read_crate_metadata_missing() {
        let (n, v, d) = read_crate_metadata(std::path::Path::new("/nonexistent/Cargo.toml"));
        assert!(n.is_none());
        assert!(v.is_none());
        assert!(d.is_none());
    }

    #[test]
    fn resolve_crate_display_name_with_toml() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("crates/my-lib")).unwrap();
        std::fs::write(
            root.join("crates/my-lib/Cargo.toml"),
            "[package]\nname = \"ops-my-lib\"\n",
        )
        .unwrap();
        assert_eq!(
            resolve_crate_display_name("crates/my-lib", root),
            "ops-my-lib"
        );
    }

    #[test]
    fn resolve_crate_display_name_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            resolve_crate_display_name("crates/nothing", dir.path()),
            "Nothing"
        );
    }
}
