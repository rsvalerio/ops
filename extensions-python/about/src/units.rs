//! Python `project_units` data provider.
//!
//! Reads `[tool.uv.workspace].members` globs from the root `pyproject.toml`
//! and resolves per-package metadata from each member's `pyproject.toml`.

use std::path::Path;

use ops_about::cards::format_unit_name;
use ops_core::project_identity::ProjectUnit;
use ops_extension::{Context, DataProvider, DataProviderError};
use serde::Deserialize;

pub(crate) const PROVIDER_NAME: &str = "project_units";

pub(crate) struct PythonUnitsProvider;

impl DataProvider for PythonUnitsProvider {
    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let units = collect_units(ctx.working_directory.as_path());
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
    members: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
}

fn read_workspace_members(root: &Path) -> Vec<(String, String)> {
    // DUP-3 / TASK-0816: share the parsed `toml::Value` with the identity
    // provider via the per-process cache rather than re-reading and
    // re-parsing the same `pyproject.toml`.
    // PERF-3 / TASK-0854: parse directly from the cached raw text into
    // the workspace shape, skipping the toml::Value intermediate clone.
    let Some(text) = ops_about::manifest_cache::for_filename("pyproject.toml").read(root) else {
        return Vec::new();
    };
    let raw: RawRoot = match toml::from_str(&text) {
        Ok(r) => r,
        Err(e) => {
            // ERR-7 / TASK-0974: include the manifest path so multi-root
            // `ops about` runs can attribute the parse failure. Debug-format
            // so embedded newlines / ANSI cannot forge log records.
            tracing::warn!(
                path = ?root.join("pyproject.toml").display(),
                error = %e,
                "failed to project pyproject.toml into workspace shape"
            );
            return Vec::new();
        }
    };
    raw.tool
        .and_then(|t| t.uv)
        .and_then(|u| u.workspace)
        .map(|w| {
            ops_about::workspace::resolve_member_globs(
                &w.members,
                &w.exclude,
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
            // DUP-3 / TASK-0987: call the shared `parse_package_metadata`
            // directly so the per-stack `PackageProbe` lives next to the
            // deserialiser, not behind a parallel shim function.
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
            let mut unit = ProjectUnit::new(
                meta.name.unwrap_or_else(|| format_unit_name(&member)),
                member,
            );
            unit.version = meta.version;
            unit.description = meta.description;
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

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }

    /// ERR-7 / TASK-0974: workspace-shape parse warn now includes the
    /// manifest path. Pin the formatter so embedded newlines / ANSI in an
    /// attacker-controlled checkout path cannot forge log records.
    #[test]
    fn workspace_pyproject_path_debug_escapes_control_characters() {
        let p = Path::new("a\nb\u{1b}[31mc/pyproject.toml");
        let rendered = format!("{:?}", p.display());
        assert!(!rendered.contains('\n'));
        assert!(!rendered.contains('\u{1b}'));
        assert!(rendered.contains("\\n"));
    }

    #[test]
    fn no_workspace_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("pyproject.toml"),
            "[project]\nname = \"single\"\nversion = \"0.1.0\"\n",
        );
        assert!(collect_units(dir.path()).is_empty());
    }

    #[test]
    fn workspace_glob_members() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("pyproject.toml"),
            r#"
[project]
name = "root"
version = "0.0.0"

[tool.uv.workspace]
members = ["packages/*"]
"#,
        );
        write(
            &dir.path().join("packages/alpha/pyproject.toml"),
            "[project]\nname = \"alpha\"\nversion = \"1.0.0\"\ndescription = \"A\"\n",
        );
        write(
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
        write(
            &dir.path().join("pyproject.toml"),
            r#"
[project]
name = "root"

[tool.uv.workspace]
members = ["libs/mylib"]
"#,
        );
        write(
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
        write(
            &dir.path().join("pyproject.toml"),
            r#"
[project]
name = "root"

[tool.uv.workspace]
members = ["packages/*"]
exclude = ["packages/internal-*"]
"#,
        );
        write(
            &dir.path().join("packages/public/pyproject.toml"),
            "[project]\nname = \"public\"\n",
        );
        write(
            &dir.path().join("packages/internal-thing/pyproject.toml"),
            "[project]\nname = \"internal-thing\"\n",
        );
        let units = collect_units(dir.path());
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].name, "public");
    }

    #[test]
    fn falls_back_to_dir_name_when_no_project_table() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("pyproject.toml"),
            r#"
[tool.uv.workspace]
members = ["packages/quiet"]
"#,
        );
        // Subpackage exists but has no [project] table.
        write(
            &dir.path().join("packages/quiet/pyproject.toml"),
            "[tool.something]\nkey = \"v\"\n",
        );
        let units = collect_units(dir.path());
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].name, "Quiet");
    }
}
