//! Go `project_units` data provider.
//!
//! Reads `go.work` / `go.mod` to build a list of [`ProjectUnit`] entries
//! describing each module. LOC/file counts are enriched by the generic
//! `ops_about::run_about_units` runner.

use std::path::Path;

use ops_about::cards::format_unit_name;
use ops_core::project_identity::ProjectUnit;
use ops_extension::{Context, DataProvider, DataProviderError};

pub(crate) const PROVIDER_NAME: &str = "project_units";

pub(crate) struct GoUnitsProvider;

impl DataProvider for GoUnitsProvider {
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
    if let Some(dirs) = workspace_use_dirs(cwd) {
        dirs.into_iter()
            .map(|dir| {
                let normalized = normalize_module_path(&dir);
                let mod_path = cwd.join(&normalized);
                let (module, go_version) = read_mod_info(&mod_path);
                let name = last_segment(module.as_deref())
                    .unwrap_or_else(|| format_unit_name(&normalized));
                ProjectUnit {
                    name,
                    path: normalized,
                    version: go_version,
                    description: module,
                    ..Default::default()
                }
            })
            .collect()
    } else {
        let (module, go_version) = read_mod_info(cwd);
        match module {
            Some(m) => vec![ProjectUnit {
                name: last_segment(Some(&m)).unwrap_or_else(|| m.clone()),
                // Empty path matches every file in `tokei_files` via starts_with.
                path: String::new(),
                version: go_version,
                description: Some(m),
                ..Default::default()
            }],
            None => vec![],
        }
    }
}

/// Normalize a `go.work` use-directive entry so it matches `tokei_files.file`
/// paths (which are recorded relative to cwd with no `./` prefix).
/// `.` → empty string (signals project-root module; enriched via project-wide
/// stats instead of the per-crate SQL join).
fn normalize_module_path(dir: &str) -> String {
    let trimmed = dir.trim_start_matches("./").trim_end_matches('/');
    if trimmed == "." {
        String::new()
    } else {
        trimmed.to_string()
    }
}

fn last_segment(module: Option<&str>) -> Option<String> {
    module
        .and_then(|m| m.rsplit('/').next())
        .map(|s| s.to_string())
}

fn workspace_use_dirs(root: &Path) -> Option<Vec<String>> {
    crate::go_work::parse_use_dirs(root)
}

fn read_mod_info(dir: &Path) -> (Option<String>, Option<String>) {
    let path = dir.join("go.mod");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::debug!(path = %path.display(), error = %e, "failed to read go.mod");
            }
            return (None, None);
        }
    };
    let mut module = None;
    let mut go_version = None;
    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("module ") {
            module = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("go ") {
            go_version = Some(rest.trim().to_string());
        }
        if module.is_some() && go_version.is_some() {
            break;
        }
    }
    (module, go_version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_use_dirs_multi() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse (\n\t./api\n\t./cmd\n)\n",
        )
        .unwrap();
        let dirs = workspace_use_dirs(dir.path()).unwrap();
        assert_eq!(dirs, vec!["./api", "./cmd"]);
    }

    #[test]
    fn read_mod_info_basic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module github.com/user/myapp\n\ngo 1.22\n",
        )
        .unwrap();
        let (module, go_version) = read_mod_info(dir.path());
        assert_eq!(module.as_deref(), Some("github.com/user/myapp"));
        assert_eq!(go_version.as_deref(), Some("1.22"));
    }

    #[test]
    fn collect_units_workspace() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse (\n\t./api\n\t./cmd\n)\n",
        )
        .unwrap();
        let api = dir.path().join("api");
        std::fs::create_dir(&api).unwrap();
        std::fs::write(api.join("go.mod"), "module example.com/api\n\ngo 1.21\n").unwrap();
        let cmd = dir.path().join("cmd");
        std::fs::create_dir(&cmd).unwrap();
        std::fs::write(cmd.join("go.mod"), "module example.com/cmd\n\ngo 1.22\n").unwrap();

        let units = collect_units(dir.path());
        assert_eq!(units.len(), 2);
        assert_eq!(units[0].name, "api");
        assert_eq!(units[0].description.as_deref(), Some("example.com/api"));
        assert_eq!(units[1].version.as_deref(), Some("1.22"));
    }

    #[test]
    fn collect_units_single_mod() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module github.com/user/app\n\ngo 1.23\n",
        )
        .unwrap();
        let units = collect_units(dir.path());
        assert_eq!(units.len(), 1);
        // Empty path signals project-root module; enrichment uses project-wide stats.
        assert_eq!(units[0].path, "");
        assert_eq!(units[0].name, "app");
    }

    #[test]
    fn normalize_strips_dot_slash_prefix() {
        assert_eq!(
            normalize_module_path("./staging/src/k8s.io/api"),
            "staging/src/k8s.io/api"
        );
        assert_eq!(normalize_module_path("./api/"), "api");
        assert_eq!(normalize_module_path("."), "");
        assert_eq!(normalize_module_path("pkg/foo"), "pkg/foo");
    }

    #[test]
    fn collect_units_empty() {
        let dir = tempfile::tempdir().unwrap();
        let units = collect_units(dir.path());
        assert!(units.is_empty());
    }
}
