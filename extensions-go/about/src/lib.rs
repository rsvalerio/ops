//! Go stack `project_identity` provider.
//!
//! Parses `go.mod` for module name and Go version, and `go.work` for
//! workspace modules. Provides a [`ProjectIdentity`] for the generic about command.

use std::path::Path;

use ops_core::project_identity::ProjectIdentity;
use ops_extension::{Context, DataProvider, DataProviderError, ExtensionType};

const NAME: &str = "about-go";
const DESCRIPTION: &str = "Go project identity";
const SHORTNAME: &str = "about-go";
const DATA_PROVIDER_NAME: &str = "project_identity";

pub struct AboutGoExtension;

ops_extension::impl_extension! {
    AboutGoExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE,
    stack: Some(ops_extension::Stack::Go),
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_data_providers: |_self, registry| {
        registry.register(DATA_PROVIDER_NAME, Box::new(GoIdentityProvider));
    },
    factory: GO_ABOUT_FACTORY = |_, _| {
        Some((NAME, Box::new(AboutGoExtension)))
    },
}

struct GoIdentityProvider;

impl DataProvider for GoIdentityProvider {
    fn name(&self) -> &'static str {
        DATA_PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let cwd = ctx.working_directory.clone();
        let go_mod = parse_go_mod(&cwd);
        let go_work = parse_go_work(&cwd);

        let module_path = go_mod.as_ref().map(|m| m.module.as_str());

        // Use last segment of module path as name, e.g. "github.com/openbao/openbao" → "openbao"
        let name = module_path
            .and_then(|m| m.rsplit('/').next())
            .unwrap_or_else(|| dir_name(&cwd))
            .to_string();

        let go_version = go_mod.as_ref().and_then(|m| m.go_version.clone());
        let stack_detail = go_version.map(|v| format!("Go {v}"));

        // Module count from go.work (workspace) or None for single-module projects.
        let module_count = go_work.as_ref().map(|w| w.use_dirs.len());

        let identity = ProjectIdentity {
            name,
            version: None,
            description: None,
            stack_label: "Go".to_string(),
            stack_detail,
            license: None,
            project_path: cwd.display().to_string(),
            module_count,
            module_label: "modules".to_string(),
            loc: None,
            file_count: None,
            authors: vec![],
            repository: None,
        };

        serde_json::to_value(&identity).map_err(DataProviderError::from)
    }
}

// --- go.mod parsing ---

struct GoMod {
    module: String,
    go_version: Option<String>,
}

fn parse_go_mod(project_root: &Path) -> Option<GoMod> {
    let content = std::fs::read_to_string(project_root.join("go.mod")).ok()?;
    let mut module = None;
    let mut go_version = None;

    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("module ") {
            module = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("go ") {
            go_version = Some(rest.trim().to_string());
        }
        // Stop after we have both — they're always near the top.
        if module.is_some() && go_version.is_some() {
            break;
        }
    }

    module.map(|m| GoMod {
        module: m,
        go_version,
    })
}

// --- go.work parsing ---

struct GoWork {
    use_dirs: Vec<String>,
}

fn parse_go_work(project_root: &Path) -> Option<GoWork> {
    let content = std::fs::read_to_string(project_root.join("go.work")).ok()?;
    let mut use_dirs = Vec::new();
    let mut in_use_block = false;

    for line in content.lines() {
        let line = line.trim();
        if line == "use (" {
            in_use_block = true;
            continue;
        }
        if in_use_block {
            if line == ")" {
                in_use_block = false;
                continue;
            }
            if !line.is_empty() && !line.starts_with("//") {
                use_dirs.push(line.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("use ") {
            // Single-line: `use ./mymod`
            let dir = rest.trim();
            if !dir.starts_with('(') {
                use_dirs.push(dir.to_string());
            }
        }
    }

    if use_dirs.is_empty() {
        None
    } else {
        Some(GoWork { use_dirs })
    }
}

fn dir_name(path: &Path) -> &str {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_go_mod_basic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module github.com/user/myapp\n\ngo 1.21\n",
        )
        .unwrap();
        let m = parse_go_mod(dir.path()).unwrap();
        assert_eq!(m.module, "github.com/user/myapp");
        assert_eq!(m.go_version, Some("1.21".to_string()));
    }

    #[test]
    fn parse_go_mod_no_go_version() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.mod"), "module example.com/foo\n").unwrap();
        let m = parse_go_mod(dir.path()).unwrap();
        assert_eq!(m.module, "example.com/foo");
        assert!(m.go_version.is_none());
    }

    #[test]
    fn parse_go_mod_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(parse_go_mod(dir.path()).is_none());
    }

    #[test]
    fn parse_go_work_multi_use() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse (\n\t./api\n\t./cmd\n\t./sdk\n)\n",
        )
        .unwrap();
        let w = parse_go_work(dir.path()).unwrap();
        assert_eq!(w.use_dirs, vec!["./api", "./cmd", "./sdk"]);
    }

    #[test]
    fn parse_go_work_single_use() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.work"), "go 1.21\nuse ./mymod\n").unwrap();
        let w = parse_go_work(dir.path()).unwrap();
        assert_eq!(w.use_dirs, vec!["./mymod"]);
    }

    #[test]
    fn parse_go_work_missing() {
        let dir = tempfile::tempdir().unwrap();
        assert!(parse_go_work(dir.path()).is_none());
    }

    #[test]
    fn name_from_module_path() {
        assert_eq!(
            "github.com/openbao/openbao".rsplit('/').next().unwrap(),
            "openbao"
        );
    }
}
