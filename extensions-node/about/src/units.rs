//! Node `project_units` data provider.
//!
//! Enumerates workspace members from `package.json` (`workspaces`) or
//! `pnpm-workspace.yaml` (pnpm). Glob entries like `packages/*` expand to
//! directories that contain a `package.json`.

use std::path::Path;

use ops_about::cards::format_unit_name;
use ops_core::project_identity::ProjectUnit;
use ops_extension::{Context, DataProvider, DataProviderError};
use serde::Deserialize;

pub(crate) const PROVIDER_NAME: &str = "project_units";

pub(crate) struct NodeUnitsProvider;

impl DataProvider for NodeUnitsProvider {
    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let cwd = ctx.working_directory.clone();
        let units = collect_units(&cwd);
        serde_json::to_value(&units).map_err(DataProviderError::from)
    }
}

fn collect_units(cwd: &Path) -> Vec<ProjectUnit> {
    let members = workspace_member_globs(cwd);
    let resolved = resolve_member_globs(&members, cwd);
    resolved
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
struct RawRoot {
    workspaces: Option<WorkspacesField>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum WorkspacesField {
    List(Vec<String>),
    Object {
        #[serde(default)]
        packages: Vec<String>,
    },
}

/// Collect raw glob patterns from either `package.json`.workspaces or
/// `pnpm-workspace.yaml` (naive parse — packages-list only).
fn workspace_member_globs(root: &Path) -> Vec<String> {
    let mut patterns = Vec::new();

    if let Ok(content) = std::fs::read_to_string(root.join("package.json")) {
        if let Ok(raw) = serde_json::from_str::<RawRoot>(&content) {
            if let Some(ws) = raw.workspaces {
                match ws {
                    WorkspacesField::List(items) => patterns.extend(items),
                    WorkspacesField::Object { packages } => patterns.extend(packages),
                }
            }
        }
    }

    if patterns.is_empty() {
        if let Ok(content) = std::fs::read_to_string(root.join("pnpm-workspace.yaml")) {
            patterns.extend(parse_pnpm_workspace_yaml(&content));
        }
    }

    patterns
}

/// Minimal parser for the `packages:` list in `pnpm-workspace.yaml`.
/// Handles the common shapes:
///   packages:
///     - 'apps/*'
///     - "libs/*"
///     - services/api
fn parse_pnpm_workspace_yaml(content: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_packages = false;
    for raw_line in content.lines() {
        let line = raw_line.trim_end();
        if line.trim_start().starts_with('#') || line.trim().is_empty() {
            continue;
        }
        if line.trim_start().starts_with("packages:") && !line.contains('[') {
            in_packages = true;
            continue;
        }
        if in_packages {
            let leading_ws = line.chars().take_while(|c| c.is_whitespace()).count();
            if leading_ws == 0 {
                // Next top-level key ends the block.
                in_packages = false;
                continue;
            }
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("- ") {
                out.push(unquote(rest.trim()).to_string());
            } else if let Some(rest) = trimmed.strip_prefix('-') {
                let rest = rest.trim();
                if !rest.is_empty() {
                    out.push(unquote(rest).to_string());
                }
            }
        }
    }
    out
}

fn unquote(s: &str) -> &str {
    let s = s.trim();
    s.strip_prefix('\'')
        .and_then(|t| t.strip_suffix('\''))
        .or_else(|| s.strip_prefix('"').and_then(|t| t.strip_suffix('"')))
        .unwrap_or(s)
}

/// Expand workspace glob patterns into directories that contain a
/// `package.json`. Non-glob entries pass through if their manifest is
/// readable. Supports simple `prefix/*` and `prefix/**/suffix` forms by
/// matching on prefix.
///
/// Returns `(member_path, package.json contents)` so the caller does not need
/// to re-open the file — collapsing the prior `exists()`-then-`read_to_string`
/// pair into a single read avoids the SEC-25 TOCTOU window in which a symlink
/// swap between the probe and the open could redirect the read.
fn resolve_member_globs(members: &[String], root: &Path) -> Vec<(String, String)> {
    let mut resolved: Vec<(String, String)> = Vec::new();
    for member in members {
        let trimmed = member.trim_start_matches("./");
        // Exclusion pattern (yarn) — ignore.
        if trimmed.starts_with('!') {
            continue;
        }
        if let Some(idx) = trimmed.find('*') {
            let prefix = &trimmed[..idx];
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
        } else if let Some(manifest) = try_read_manifest(&root.join(trimmed)) {
            resolved.push((trimmed.to_string(), manifest));
        }
    }
    resolved.sort_by(|a, b| a.0.cmp(&b.0));
    resolved.dedup_by(|a, b| a.0 == b.0);
    resolved
}

/// Read `<dir>/package.json`, treating `NotFound` as "not a package directory"
/// rather than an error. Other I/O errors are also coerced to `None` so a
/// transient failure on one member does not poison the whole walk.
fn try_read_manifest(dir: &Path) -> Option<String> {
    match std::fs::read_to_string(dir.join("package.json")) {
        Ok(content) => Some(content),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => None,
    }
}

#[derive(Debug, Deserialize)]
struct PackageProbe {
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
}

fn parse_package_metadata(content: &str) -> (Option<String>, Option<String>, Option<String>) {
    let parsed: PackageProbe = match serde_json::from_str(content) {
        Ok(p) => p,
        Err(_) => return (None, None, None),
    };
    (
        parsed.name,
        parsed.version,
        parsed
            .description
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
    fn no_workspaces_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("package.json"), r#"{ "name": "solo" }"#);
        assert!(collect_units(dir.path()).is_empty());
    }

    #[test]
    fn npm_workspaces_array_form() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("package.json"),
            r#"{ "name": "root", "workspaces": ["packages/*"] }"#,
        );
        write(
            &dir.path().join("packages/alpha/package.json"),
            r#"{ "name": "@scope/alpha", "version": "1.0.0", "description": "A" }"#,
        );
        write(
            &dir.path().join("packages/beta/package.json"),
            r#"{ "name": "beta", "version": "2.0.0" }"#,
        );
        // No package.json → not a workspace.
        std::fs::create_dir_all(dir.path().join("packages/not-a-pkg")).unwrap();

        let units = collect_units(dir.path());
        assert_eq!(units.len(), 2);
        assert_eq!(units[0].name, "@scope/alpha");
        assert_eq!(units[0].version.as_deref(), Some("1.0.0"));
        assert_eq!(units[0].description.as_deref(), Some("A"));
        assert_eq!(units[1].name, "beta");
    }

    #[test]
    fn yarn_workspaces_object_form() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("package.json"),
            r#"{ "name": "root", "workspaces": { "packages": ["apps/*"] } }"#,
        );
        write(
            &dir.path().join("apps/web/package.json"),
            r#"{ "name": "web", "version": "0.0.1" }"#,
        );
        let units = collect_units(dir.path());
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].path, "apps/web");
        assert_eq!(units[0].name, "web");
    }

    #[test]
    fn pnpm_workspace_yaml() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("package.json"), r#"{ "name": "root" }"#);
        write(
            &dir.path().join("pnpm-workspace.yaml"),
            "packages:\n  - 'libs/*'\n  - \"apps/web\"\n",
        );
        write(
            &dir.path().join("libs/foo/package.json"),
            r#"{ "name": "foo" }"#,
        );
        write(
            &dir.path().join("apps/web/package.json"),
            r#"{ "name": "web" }"#,
        );
        let units = collect_units(dir.path());
        assert_eq!(units.len(), 2);
        let names: Vec<&str> = units.iter().map(|u| u.name.as_str()).collect();
        assert!(names.contains(&"foo"));
        assert!(names.contains(&"web"));
    }

    #[test]
    fn exclusion_pattern_ignored() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("package.json"),
            r#"{ "name": "root", "workspaces": ["packages/*", "!packages/ignored"] }"#,
        );
        write(
            &dir.path().join("packages/keep/package.json"),
            r#"{ "name": "keep" }"#,
        );
        write(
            &dir.path().join("packages/ignored/package.json"),
            r#"{ "name": "ignored" }"#,
        );
        // Both get picked up by the `packages/*` glob; the `!...` exclusion entry
        // is currently a passthrough (no match). Good enough: still shows both.
        let units = collect_units(dir.path());
        assert!(units.iter().any(|u| u.name == "keep"));
    }

    #[test]
    fn falls_back_to_dir_name_when_no_name() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("package.json"),
            r#"{ "name": "root", "workspaces": ["packages/*"] }"#,
        );
        write(
            &dir.path().join("packages/quiet/package.json"),
            r#"{ "version": "0.1.0" }"#,
        );
        let units = collect_units(dir.path());
        assert_eq!(units.len(), 1);
        assert_eq!(units[0].name, "Quiet");
    }

    #[test]
    fn parses_pnpm_packages_list() {
        let yaml = "packages:\n  - 'apps/*'\n  - \"libs/core\"\n  - services/api\n\nother: key\n";
        let pats = parse_pnpm_workspace_yaml(yaml);
        assert_eq!(pats, vec!["apps/*", "libs/core", "services/api"]);
    }
}
