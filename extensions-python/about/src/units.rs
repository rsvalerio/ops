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
        let cwd = ctx.working_directory.clone();
        let units = collect_units(&cwd);
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
    let content = match std::fs::read_to_string(root.join("pyproject.toml")) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let raw: RawRoot = match toml::from_str(&content) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    raw.tool
        .and_then(|t| t.uv)
        .and_then(|u| u.workspace)
        .map(|w| (w.members, w.exclude))
        .map(|(members, exclude)| resolve_member_globs(&members, &exclude, root))
        .unwrap_or_default()
}

/// Expand uv workspace member globs (e.g. `packages/*`) into directories that
/// contain a `pyproject.toml`. Non-glob entries pass through if their manifest
/// is readable. Entries matching any `exclude` glob are filtered out.
///
/// Returns `(member_path, pyproject.toml contents)` so the caller does not
/// need to re-open the file. Collapsing the previous `exists()`-then-
/// `read_to_string` pair closes the SEC-25 TOCTOU window where a symlink swap
/// between the probe and the open could redirect the read.
fn resolve_member_globs(
    members: &[String],
    exclude: &[String],
    root: &Path,
) -> Vec<(String, String)> {
    let mut resolved: Vec<(String, String)> = Vec::new();
    for member in members {
        if let Some(idx) = member.find('*') {
            let prefix = &member[..idx];
            let parent = root.join(prefix);
            if let Ok(entries) = std::fs::read_dir(&parent) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_dir() {
                        continue;
                    }
                    if let Some(manifest) = try_read_manifest(&path) {
                        if let Ok(rel) = path.strip_prefix(root) {
                            resolved.push((rel.to_string_lossy().to_string(), manifest));
                        }
                    }
                }
            }
        } else if let Some(manifest) = try_read_manifest(&root.join(member)) {
            resolved.push((member.clone(), manifest));
        }
    }
    resolved.retain(|(m, _)| !exclude.iter().any(|pat| matches_exclude(pat, m)));
    resolved.sort_by(|a, b| a.0.cmp(&b.0));
    resolved.dedup_by(|a, b| a.0 == b.0);
    resolved
}

/// Read `<dir>/pyproject.toml`, mapping `NotFound` to "not a package
/// directory" without surfacing it as an error. Other I/O errors are also
/// coerced to `None` so a transient failure on one member does not break the
/// whole walk.
fn try_read_manifest(dir: &Path) -> Option<String> {
    match std::fs::read_to_string(dir.join("pyproject.toml")) {
        Ok(content) => Some(content),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => None,
    }
}

fn matches_exclude(pattern: &str, candidate: &str) -> bool {
    if let Some(idx) = pattern.find('*') {
        candidate.starts_with(&pattern[..idx])
    } else {
        pattern == candidate
    }
}

fn collect_units(cwd: &Path) -> Vec<ProjectUnit> {
    let members = read_workspace_members(cwd);
    members
        .into_iter()
        .map(|(member, manifest)| {
            let (name, version, description) = parse_package_metadata(&manifest);
            ProjectUnit {
                name: name.unwrap_or_else(|| format_unit_name(&member)),
                path: member,
                version,
                description,
                ..Default::default()
            }
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

fn parse_package_metadata(content: &str) -> (Option<String>, Option<String>, Option<String>) {
    let parsed: PackageProbe = match toml::from_str(content) {
        Ok(p) => p,
        Err(_) => return (None, None, None),
    };
    let p = match parsed.project {
        Some(p) => p,
        None => return (None, None, None),
    };
    (
        p.name,
        p.version,
        p.description
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
    )
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
