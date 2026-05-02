//! Rust-specific `project_units` data provider.
//!
//! Reads `[workspace].members` from Cargo.toml and per-crate Cargo manifests
//! for display metadata. LOC/file counts are enriched by the generic
//! `run_about_units` runner when DuckDB is available.

use ops_about::cards::format_unit_name;
use ops_cargo_toml::CargoToml;
use ops_core::project_identity::ProjectUnit;
use ops_extension::{Context, DataProvider, DataProviderError};

use crate::query::{load_workspace_manifest, log_manifest_load_failure};

/// Subset of crate manifest metadata used by the `project_units` provider.
///
/// FN-4 (TASK-0805): named struct so adding a field cannot silently shift
/// positions in tuple destructures at call sites.
#[derive(Debug, Default, Clone)]
pub(crate) struct CrateMetadata {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
}

pub(crate) const PROVIDER_NAME: &str = "project_units";

pub(crate) struct RustUnitsProvider;

impl DataProvider for RustUnitsProvider {
    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let cwd = ctx.working_directory.clone();

        let manifest = match load_workspace_manifest(ctx) {
            Ok(m) => m,
            Err(e) => {
                log_manifest_load_failure(&e);
                return Ok(serde_json::to_value(Vec::<ProjectUnit>::new())?);
            }
        };
        let members: &[String] = manifest
            .workspace
            .as_ref()
            .map_or(&[][..], |ws| ws.members.as_slice());

        // Per-crate dep counts from DuckDB (Rust-specific, keyed by package name).
        // ERR-2 / TASK-0376: query failures route through `query_or_warn` so
        // they don't manifest as a silent "no deps" on a misconfigured DB.
        let dep_counts: std::collections::HashMap<String, i64> = match ops_duckdb::get_db(ctx) {
            None => std::collections::HashMap::new(),
            Some(db) => ops_duckdb::sql::query_or_warn(
                "query_crate_dep_counts",
                "per-crate dep_counts will be empty",
                std::collections::HashMap::<String, i64>::new(),
                || ops_duckdb::sql::query_crate_dep_counts(db),
            ),
        };

        let mut sorted_members: Vec<&str> = members.iter().map(String::as_str).collect();
        sorted_members.sort_unstable();

        let units: Vec<ProjectUnit> = sorted_members
            .into_iter()
            .map(|member| {
                let crate_toml = cwd.join(member).join("Cargo.toml");
                let CrateMetadata {
                    name: pkg_name,
                    version,
                    description,
                } = read_crate_metadata(&crate_toml);
                // ERR-2 (TASK-0804): when the manifest is missing or
                // unparseable, the package name is unknown — return None for
                // dep_count rather than masquerading the lookup miss as a
                // legitimate "no deps" answer.
                let dep_count = match pkg_name.as_deref() {
                    Some(pn) => dep_counts.get(pn).copied(),
                    None => {
                        tracing::debug!(
                            member,
                            "no package name resolved for member; dep_count unavailable"
                        );
                        None
                    }
                };
                let name = format_unit_name(member);

                let mut unit = ProjectUnit::new(name, member.to_string());
                unit.version = version;
                unit.description = description;
                unit.dep_count = dep_count;
                unit
            })
            .collect();

        serde_json::to_value(&units).map_err(DataProviderError::from)
    }
}

/// Read package name, version, and description from a crate's Cargo.toml.
///
/// Returns an all-`None` [`CrateMetadata`] on read or parse failure. NotFound
/// reads are silent (an absent member manifest is expected during workspace
/// globbing); other read errors are logged at `debug` and parse errors at
/// `warn` so a malformed Cargo.toml shows up in logs instead of silently
/// producing an empty unit (TASK-0377).
///
/// DUP-3 (TASK-0806): delegates to `ops_cargo_toml::CargoToml::parse` so this
/// extension does not maintain a second TOML parser for the same manifest
/// shape.
pub(crate) fn read_crate_metadata(crate_toml_path: &std::path::Path) -> CrateMetadata {
    // SEC-33 (TASK-0926): cap the per-crate manifest read; this fans out across
    // every workspace member declared by the root Cargo.toml.
    let content = match ops_core::text::read_capped_to_string(crate_toml_path) {
        Ok(c) => c,
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::debug!(
                    path = %crate_toml_path.display(),
                    error = %e,
                    "failed to read crate manifest"
                );
            }
            return CrateMetadata::default();
        }
    };

    let parsed = match CargoToml::parse(&content) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(
                path = %crate_toml_path.display(),
                error = %e,
                "failed to parse crate manifest as TOML"
            );
            return CrateMetadata::default();
        }
    };

    let name = parsed.package_name().map(str::to_string);
    let version = parsed.package_version().map(str::to_string);
    let description = parsed
        .package
        .as_ref()
        .and_then(|p| p.description.as_str())
        .map(str::to_string);

    CrateMetadata {
        name,
        version,
        description,
    }
}

/// Resolve display name for a member by reading its Cargo.toml, falling back
/// to the capitalized last path segment.
pub(crate) fn resolve_crate_display_name(member: &str, workspace_root: &std::path::Path) -> String {
    let toml_path = workspace_root.join(member).join("Cargo.toml");
    read_crate_metadata(&toml_path)
        .name
        .unwrap_or_else(|| format_unit_name(member))
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
        let meta = read_crate_metadata(&path);
        assert_eq!(meta.name.as_deref(), Some("foo"));
        assert_eq!(meta.version.as_deref(), Some("1.0.0"));
        assert_eq!(meta.description.as_deref(), Some("a foo"));
    }

    #[test]
    fn read_crate_metadata_missing() {
        let meta = read_crate_metadata(std::path::Path::new("/nonexistent/Cargo.toml"));
        assert!(meta.name.is_none());
        assert!(meta.version.is_none());
        assert!(meta.description.is_none());
    }

    /// TASK-0377 AC#2: a malformed Cargo.toml returns an empty `CrateMetadata`
    /// and should not crash. Verifying the warn-log fires would require a
    /// tracing subscriber; we settle for asserting the function is total over
    /// invalid TOML so the warn branch is at least exercised.
    #[test]
    fn read_crate_metadata_malformed_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Cargo.toml");
        std::fs::write(&path, "[package\nname = \"unterminated\n").unwrap();
        let meta = read_crate_metadata(&path);
        assert!(meta.name.is_none());
        assert!(meta.version.is_none());
        assert!(meta.description.is_none());
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
