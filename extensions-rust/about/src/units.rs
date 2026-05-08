//! Rust-specific `project_units` data provider.
//!
//! Reads `[workspace].members` from Cargo.toml and per-crate Cargo manifests
//! for display metadata. LOC/file counts are enriched by the generic
//! `run_about_units` runner when DuckDB is available.

use ops_about::cards::format_unit_name;
use ops_cargo_toml::CargoToml;
use ops_core::project_identity::ProjectUnit;
use ops_extension::{Context, DataProvider, DataProviderError};

use crate::query::{
    load_workspace_manifest, log_manifest_load_failure, member_path_is_workspace_safe,
};

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
        // ERR-1 / TASK-1076: read the resolved-members sibling on
        // `LoadedManifest`. The cached `manifest.workspace.members` now
        // preserves the original glob spec (e.g. `["crates/*"]`) verbatim;
        // only `resolved_members()` returns the post-expansion list this
        // provider needs.
        let members: &[String] = manifest.resolved_members();

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
            .filter(|member| {
                // SEC-14 / TASK-1246 AC #1: defence-in-depth — even though
                // `resolved_workspace_members` already filters absolute /
                // `..`-segment entries, re-validate here so a future caller
                // bypassing that helper (custom enrichment, test harness)
                // cannot drive `cwd.join(member)` at arbitrary filesystem
                // locations. The warn is intentionally cheap (rare path
                // under normal workspaces) and matches the helper's
                // breadcrumb shape so an attacker-controlled member surfaces
                // exactly once per provider invocation.
                if !member_path_is_workspace_safe(member) {
                    tracing::warn!(
                        member = %member,
                        "SEC-14 / TASK-1246: rejecting absolute or `..` workspace member in units provider"
                    );
                    return false;
                }
                true
            })
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
                // ERR-7 (TASK-0977): Debug-format path/error so an
                // attacker-controlled workspace member name with embedded
                // newlines / ANSI escapes cannot forge log records.
                tracing::debug!(
                    path = ?crate_toml_path.display(),
                    error = ?e,
                    "failed to read crate manifest"
                );
            }
            return CrateMetadata::default();
        }
    };

    let parsed = match CargoToml::parse(&content) {
        Ok(p) => p,
        Err(e) => {
            // ERR-7 (TASK-0977): Debug-format path/error so an
            // attacker-controlled workspace member name with embedded
            // newlines / ANSI escapes cannot forge log records.
            tracing::warn!(
                path = ?crate_toml_path.display(),
                error = ?e,
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
///
/// SEC-14 / TASK-1246: rejects absolute and `..`-traversal member entries
/// before any join and falls back to the formatted member name. `Path::join`
/// discards `workspace_root` when `member` is absolute and walks parents on
/// `..`, which would otherwise drive `read_capped_to_string` and tracing
/// breadcrumbs at any filesystem location.
pub(crate) fn resolve_crate_display_name(member: &str, workspace_root: &std::path::Path) -> String {
    if !member_path_is_workspace_safe(member) {
        tracing::warn!(
            member = %member,
            "SEC-14 / TASK-1246: rejecting absolute or `..` workspace member in display-name resolver"
        );
        return format_unit_name(member);
    }
    let toml_path = workspace_root.join(member).join("Cargo.toml");
    read_crate_metadata(&toml_path)
        .name
        .unwrap_or_else(|| format_unit_name(member))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ERR-7 (TASK-0977): tracing fields for crate-manifest paths flow
    /// through the `?` formatter so an attacker-controlled workspace member
    /// path with embedded newlines / ANSI escapes cannot forge log records.
    #[test]
    fn crate_metadata_breadcrumb_debug_escapes_control_characters() {
        let p = std::path::Path::new("a\nb\u{1b}[31mc/Cargo.toml");
        let rendered = format!("{:?}", p.display());
        assert!(!rendered.contains('\n'));
        assert!(!rendered.contains('\u{1b}'));
        assert!(rendered.contains("\\n"));
    }

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

    /// SEC-14 / TASK-1246 AC #3: a workspace whose `[workspace].members`
    /// list contains absolute (`/abs`) or `..`-traversal (`../escape`)
    /// entries must produce zero `ProjectUnit`s and not drive any
    /// per-crate manifest read at the hostile location. We pin the
    /// behaviour at the units provider boundary — the AC #2 scrub in
    /// `resolved_workspace_members` plus AC #1 re-validation in
    /// `provide` together guarantee no `cwd.join(member)` reaches the
    /// adversarial path.
    #[test]
    fn provide_drops_absolute_and_traversal_members() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        // Plant a hostile manifest at the path `../escape` would resolve
        // to (the parent of `root`), so a regression that lets the entry
        // through would surface as a non-empty unit list with the planted
        // manifest's name.
        let parent = root.parent().expect("tempdir has a parent");
        let hostile = parent.join("escape");
        std::fs::create_dir_all(&hostile).unwrap();
        std::fs::write(
            hostile.join("Cargo.toml"),
            "[package]\nname = \"hostile\"\nversion = \"0.0.0\"\n",
        )
        .unwrap();
        // Restore the leaked test artefact when the test ends.
        struct Cleanup<'a>(&'a std::path::Path);
        impl Drop for Cleanup<'_> {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(self.0);
            }
        }
        let _cleanup = Cleanup(&hostile);

        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"../escape\", \"/abs\"]\n",
        )
        .unwrap();

        let mut ctx = ops_extension::Context::test_context(root.to_path_buf());
        let v = RustUnitsProvider.provide(&mut ctx).expect("provide");
        let arr = v.as_array().expect("array");
        assert!(
            arr.is_empty(),
            "absolute and `..` members must produce zero units, got: {arr:?}"
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
