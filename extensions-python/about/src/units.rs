//! Python `project_units` data provider.
//!
//! Reads `[tool.uv.workspace].members` globs from the root `pyproject.toml`
//! and resolves per-package metadata from each member's `pyproject.toml`.

use std::path::Path;

use ops_about::cards::format_unit_name;
use ops_core::project_identity::ProjectUnit;
use ops_extension::{Context, DataProvider, DataProviderError};
use serde::Deserialize;

pub const PROVIDER_NAME: &str = "project_units";

pub struct PythonUnitsProvider;

impl DataProvider for PythonUnitsProvider {
    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let units = collect_units(ctx.working_directory());
        serde_json::to_value(&units).map_err(DataProviderError::from)
    }
}

#[derive(Debug, Deserialize)]
struct RawRoot {
    tool: Option<RawTool>,
}

#[derive(Debug, Deserialize)]
struct RawTool {
    uv: Option<RawUv>,
}

#[derive(Debug, Deserialize)]
struct RawUv {
    workspace: Option<RawWorkspace>,
}

#[derive(Debug, Deserialize)]
struct RawWorkspace {
    #[serde(default)]
    members: Vec<RawGlob>,
    #[serde(default)]
    exclude: Vec<RawGlob>,
}

/// One entry of `[tool.uv.workspace].members` / `.exclude`.
///
/// A plain `Vec<String>` would make the whole workspace shape fail on a
/// single non-string element, zeroing the unit list for a manifest whose
/// remaining globs are perfectly good. Tolerating the bad element — with a
/// warn naming the field — degrades that entry alone.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawGlob {
    Pattern(String),
    Unsupported(toml::Value),
}

/// Keep the string globs and warn once per unusable entry.
fn string_globs(entries: Vec<RawGlob>, field: &str, manifest_path: &Path) -> Vec<String> {
    entries
        .into_iter()
        .filter_map(|entry| match entry {
            RawGlob::Pattern(p) => Some(p),
            RawGlob::Unsupported(value) => {
                // Debug-format the path so embedded newlines / ANSI cannot
                // forge log records.
                tracing::warn!(
                    path = ?manifest_path.display(),
                    field = %format!("tool.uv.workspace.{field}"),
                    kind = value.type_str(),
                    recovery = "skip-entry",
                    "unsupported workspace glob entry; keeping the remaining entries"
                );
                None
            }
        })
        .collect()
}

/// Resolve `[tool.uv.workspace].members` globs to concrete member dirs.
///
/// Crate-visible so the identity provider can set `module_count` from the
/// *same* resolved member list this provider builds `ProjectUnit`s from: the
/// card's packages row and the units table must count the same things, as
/// they do in `extensions-rust/about`.
pub fn read_workspace_members(root: &Path) -> Vec<(String, String)> {
    // The manifest text is shared with the identity provider through the
    // per-process cache rather than re-read, and the workspace shape is
    // parsed straight from that text with no `toml::Value` intermediate.
    let Some(text) = ops_about::manifest_cache::for_filename("pyproject.toml").read(root) else {
        return Vec::new();
    };
    let raw: RawRoot = match toml::from_str(&text) {
        Ok(r) => r,
        Err(e) => {
            // The manifest path is included so a multi-root `ops about` run
            // can attribute the failure, and Debug-formatted so embedded
            // newlines / ANSI cannot forge log records. `recovery` states the
            // degradation, as every warn in this crate does, so operators can
            // filter Python About degradations uniformly.
            tracing::warn!(
                path = ?root.join("pyproject.toml").display(),
                error = %e,
                recovery = "no-units",
                "failed to project pyproject.toml into workspace shape"
            );
            return Vec::new();
        }
    };
    let manifest_path = root.join("pyproject.toml");
    raw.tool
        .and_then(|t| t.uv)
        .and_then(|u| u.workspace)
        .map(|w| {
            ops_about::workspace::resolve_member_globs(
                &string_globs(w.members, "members", &manifest_path),
                &string_globs(w.exclude, "exclude", &manifest_path),
                root,
                "pyproject.toml",
            )
        })
        .unwrap_or_default()
}

fn collect_units(cwd: &Path) -> Vec<ProjectUnit> {
    let members = read_workspace_members(cwd);
    members
        .into_iter()
        .map(|(member, manifest)| {
            let manifest_path = cwd.join(&member).join("pyproject.toml");
            // The shared `parse_package_metadata` is called directly, so
            // the per-stack `PackageProbe` lives next to its deserialiser
            // rather than behind a parallel shim.
            let meta =
                ops_about::workspace::parse_package_metadata(&manifest_path, &manifest, |c| {
                    toml::from_str::<PackageProbe>(c).map(|p| {
                        p.project
                            .map(|p| ops_about::workspace::PackageMetadata {
                                name: p.name,
                                version: p.version,
                                description: p.description,
                            })
                            .unwrap_or_default()
                    })
                });
            // Trim and drop whitespace-only fields before constructing the
            // ProjectUnit, matching the policy the identity provider applies:
            // a whitespace-only `name` must still reach the
            // `format_unit_name` directory fallback, and a whitespace-only
            // version or description must not render as a blank bullet.
            let name = ops_about::text_util::trim_nonempty(meta.name)
                .unwrap_or_else(|| format_unit_name(&member));
            let version = ops_about::text_util::trim_nonempty(meta.version);
            let description = ops_about::text_util::trim_nonempty(meta.description);
            let mut unit = ProjectUnit::new(name, member);
            unit.version = version;
            unit.description = description;
            unit
        })
        .collect()
}

#[derive(Debug, Deserialize)]
struct PackageProbe {
    project: Option<ProjectProbe>,
}

#[derive(Debug, Deserialize)]
struct ProjectProbe {
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    // The fixture writer is the shared `ops_about::test_support::write_file`
    // the `lib.rs` test module uses, not a second local spelling of it.
    use ops_about::test_support::write_file;

    /// The workspace-shape parse warn carries the manifest path through the
    /// `?` formatter, so embedded newlines / ANSI in an attacker-controlled
    /// checkout path cannot forge log records. The assertion is shared with
    /// its `lib.rs` sibling, so tightening the helper upgrades both sites.
    #[test]
    fn workspace_pyproject_path_debug_escapes_control_characters() {
        let p = Path::new("a\nb\u{1b}[31mc/pyproject.toml");
        ops_about::test_support::assert_debug_escapes_control_chars(p.display());
    }

    #[test]
    fn no_workspace_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        write_file(
            &dir.path().join("pyproject.toml"),
            "[project]\nname = \"single\"\nversion = \"0.1.0\"\n",
        );
        assert!(collect_units(dir.path()).is_empty());
    }

    #[test]
    fn workspace_glob_members() {
        let dir = tempfile::tempdir().unwrap();
        write_file(
            &dir.path().join("pyproject.toml"),
            r#"
[project]
name = "root"
version = "0.0.0"

[tool.uv.workspace]
members = ["packages/*"]
"#,
        );
        write_file(
            &dir.path().join("packages/alpha/pyproject.toml"),
            "[project]\nname = \"alpha\"\nversion = \"1.0.0\"\ndescription = \"A\"\n",
        );
        write_file(
            &dir.path().join("packages/beta/pyproject.toml"),
            "[project]\nname = \"beta\"\nversion = \"2.0.0\"\n",
        );
        // No pyproject.toml → not a unit.
        std::fs::create_dir_all(dir.path().join("packages/not-a-pkg")).unwrap();

        let units = collect_units(dir.path());
        assert_eq!(units.len(), 2);
        assert_eq!(units[0].name, "alpha");
        assert_eq!(units[0].version.as_deref(), Some("1.0.0"));
        assert_eq!(units[0].description.as_deref(), Some("A"));
        assert_eq!(units[1].name, "beta");
    }

    #[test]
    fn workspace_explicit_member() {
        let dir = tempfile::tempdir().unwrap();
        write_file(
            &dir.path().join("pyproject.toml"),
            r#"
[project]
name = "root"

[tool.uv.workspace]
members = ["libs/mylib"]
"#,
        );
        write_file(
            &dir.path().join("libs/mylib/pyproject.toml"),
            "[project]\nname = \"mylib\"\nversion = \"0.3.0\"\n",
        );
        let units = collect_units(dir.path());
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].path, "libs/mylib");
        assert_eq!(units[0].name, "mylib");
    }

    #[test]
    fn workspace_exclude_filters_members() {
        let dir = tempfile::tempdir().unwrap();
        write_file(
            &dir.path().join("pyproject.toml"),
            r#"
[project]
name = "root"

[tool.uv.workspace]
members = ["packages/*"]
exclude = ["packages/internal-*"]
"#,
        );
        write_file(
            &dir.path().join("packages/public/pyproject.toml"),
            "[project]\nname = \"public\"\n",
        );
        write_file(
            &dir.path().join("packages/internal-thing/pyproject.toml"),
            "[project]\nname = \"internal-thing\"\n",
        );
        let units = collect_units(dir.path());
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].name, "public");
    }

    /// A member whose `name`/`version`/`description` fields are
    /// whitespace-only trims and drops to `None`, so the directory fallback
    /// fires and blank fields do not leak into rendered cards.
    #[test]
    fn whitespace_only_metadata_falls_back_and_drops_blank_fields() {
        let dir = tempfile::tempdir().unwrap();
        write_file(
            &dir.path().join("pyproject.toml"),
            r#"
[project]
name = "root"

[tool.uv.workspace]
members = ["libs/blank"]
"#,
        );
        write_file(
            &dir.path().join("libs/blank/pyproject.toml"),
            "[project]\nname = \"  \"\nversion = \"  \"\ndescription = \"  \"\n",
        );
        let units = collect_units(dir.path());
        assert_eq!(units.len(), 1);
        // Directory fallback must fire when name is whitespace-only.
        assert_ne!(units[0].name, "  ");
        assert!(units[0].version.is_none());
        assert!(units[0].description.is_none());
    }

    /// The crate doc promises that a malformed root manifest degrades to
    /// *no units* rather than looking like a project without a workspace, and
    /// says so via `tracing::warn!`. The warn half is asserted here too: it
    /// must fire, name `pyproject.toml`, and state its recovery, so deleting
    /// the warn fails this test and not just the doc promise.
    #[test]
    fn invalid_root_pyproject_yields_no_units() {
        let dir = tempfile::tempdir().unwrap();
        write_file(&dir.path().join("pyproject.toml"), "[tool.uv.workspace\n");

        let (logs, units) = ops_about::test_support::capture_tracing(tracing::Level::WARN, || {
            collect_units(dir.path())
        });

        assert!(units.is_empty());
        assert!(
            logs.contains("failed to project pyproject.toml into workspace shape"),
            "the degradation warn must fire: {logs}"
        );
        assert!(
            logs.contains("pyproject.toml"),
            "the warn must name the manifest: {logs}"
        );
        assert!(
            logs.contains("recovery=\"no-units\""),
            "the warn must state its recovery: {logs}"
        );
    }

    /// A *member* whose own manifest is unparseable still appears as a unit,
    /// falling back to the `format_unit_name` directory name — the shared
    /// `parse_package_metadata` warn-and-default path, exercised from this
    /// crate.
    #[test]
    fn invalid_member_pyproject_falls_back_to_the_directory_name() {
        let dir = tempfile::tempdir().unwrap();
        write_file(
            &dir.path().join("pyproject.toml"),
            r#"
[project]
name = "root"

[tool.uv.workspace]
members = ["packages/broken"]
"#,
        );
        write_file(
            &dir.path().join("packages/broken/pyproject.toml"),
            "[project\nname = \"broken\"\n",
        );

        let units = collect_units(dir.path());
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].path, "packages/broken");
        assert_eq!(units[0].name, format_unit_name("packages/broken"));
        assert!(units[0].version.is_none());
    }

    /// One non-string element in `members` / `exclude` degrades that entry
    /// only: it must not fail the whole workspace shape and zero a unit list
    /// whose remaining globs are perfectly good.
    #[test]
    fn non_string_workspace_glob_entry_does_not_zero_the_unit_list() {
        let dir = tempfile::tempdir().unwrap();
        write_file(
            &dir.path().join("pyproject.toml"),
            r#"
[project]
name = "root"

[tool.uv.workspace]
members = ["packages/*", 42]
exclude = ["packages/internal-*", { bad = true }]
"#,
        );
        write_file(
            &dir.path().join("packages/public/pyproject.toml"),
            "[project]\nname = \"public\"\n",
        );
        write_file(
            &dir.path().join("packages/internal-thing/pyproject.toml"),
            "[project]\nname = \"internal-thing\"\n",
        );

        let units = collect_units(dir.path());
        assert_eq!(units.len(), 1, "got: {units:?}");
        assert_eq!(units[0].name, "public");
    }

    #[test]
    fn falls_back_to_dir_name_when_no_project_table() {
        let dir = tempfile::tempdir().unwrap();
        write_file(
            &dir.path().join("pyproject.toml"),
            r#"
[tool.uv.workspace]
members = ["packages/quiet"]
"#,
        );
        // Subpackage exists but has no [project] table.
        write_file(
            &dir.path().join("packages/quiet/pyproject.toml"),
            "[tool.something]\nkey = \"v\"\n",
        );
        let units = collect_units(dir.path());
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].name, "Quiet");
    }

    /// `PROVIDER_NAME` is the key the registry indexes this provider under
    /// (`lib.rs`'s `register_data_providers`), so a typo there silently
    /// unregisters the Python packages card. Mirrors the Node crate's
    /// `units_provider_name`.
    #[test]
    fn units_provider_name() {
        assert_eq!(PythonUnitsProvider.name(), PROVIDER_NAME);
        assert_eq!(PROVIDER_NAME, "project_units");
    }

    /// Drives `PythonUnitsProvider::provide` against a uv workspace tempdir
    /// and asserts the deserialised JSON payload, so the
    /// `serde_json::to_value` step and the shape consumers read are pinned,
    /// not just the private `collect_units` helper. Mirrors the Node crate's
    /// `units_provider_serialises_workspace_members`.
    #[test]
    fn units_provider_serialises_workspace_members() {
        let dir = tempfile::tempdir().unwrap();
        write_file(
            &dir.path().join("pyproject.toml"),
            r#"
[project]
name = "root"

[tool.uv.workspace]
members = ["packages/*"]
"#,
        );
        write_file(
            &dir.path().join("packages/alpha/pyproject.toml"),
            "[project]\nname = \"alpha\"\nversion = \"1.0.0\"\ndescription = \"A\"\n",
        );

        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = PythonUnitsProvider.provide(&mut ctx).unwrap();
        let units: Vec<ProjectUnit> = serde_json::from_value(value).unwrap();

        assert_eq!(units.len(), 1, "unexpected units: {units:?}");
        assert_eq!(units[0].name, "alpha");
        assert_eq!(units[0].path, "packages/alpha");
        assert_eq!(units[0].version.as_deref(), Some("1.0.0"));
        assert_eq!(units[0].description.as_deref(), Some("A"));
    }

    /// A project with no `[tool.uv.workspace]` must
    /// serialise to an empty JSON array — not `null`, and not an error.
    /// Mirrors the Node crate's `units_provider_empty_workspace_is_empty_array`.
    #[test]
    fn units_provider_no_workspace_is_empty_array() {
        let dir = tempfile::tempdir().unwrap();
        write_file(
            &dir.path().join("pyproject.toml"),
            "[project]\nname = \"single\"\nversion = \"0.1.0\"\n",
        );

        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = PythonUnitsProvider.provide(&mut ctx).unwrap();
        assert_eq!(value, serde_json::json!([]));
        let units: Vec<ProjectUnit> = serde_json::from_value(value).unwrap();
        assert!(units.is_empty());
    }
}
