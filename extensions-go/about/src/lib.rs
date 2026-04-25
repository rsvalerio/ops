//! Go stack `project_identity` + `project_units` providers.
//!
//! Parses `go.mod` for module name, Go version, and local `replace` directives.
//! Parses `go.work` for workspace modules.

mod modules;

use std::path::Path;

use ops_core::project_identity::{base_about_fields, AboutFieldDef, ProjectIdentity};
use ops_core::text::{dir_name, for_each_trimmed_line};
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
        registry.register(modules::PROVIDER_NAME, Box::new(modules::GoUnitsProvider));
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

    fn about_fields(&self) -> Vec<AboutFieldDef> {
        base_about_fields()
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

        // Module count: workspace modules take precedence, otherwise count local replaces + 1 for main module.
        let module_count = if let Some(ref work) = go_work {
            Some(work.use_dirs.len())
        } else if let Some(ref m) = go_mod {
            let count = 1 + m.local_replaces.len();
            if count > 1 {
                Some(count)
            } else {
                None
            }
        } else {
            None
        };

        let repository = ops_git::resolve_repository_with_git_fallback(&cwd, None);

        let mut identity = ProjectIdentity::new(name, "Go", cwd.display().to_string(), "modules");
        identity.stack_detail = stack_detail;
        identity.module_count = module_count;
        identity.repository = repository;

        serde_json::to_value(&identity).map_err(DataProviderError::from)
    }
}

// --- go.mod parsing ---

struct GoMod {
    module: String,
    go_version: Option<String>,
    local_replaces: Vec<String>,
}

fn parse_go_mod(project_root: &Path) -> Option<GoMod> {
    let mut module = None;
    let mut go_version = None;
    let mut local_replaces = Vec::new();

    for_each_trimmed_line(&project_root.join("go.mod"), |line| {
        if let Some(rest) = line.strip_prefix("module ") {
            module = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("go ") {
            go_version = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("replace ") {
            // Parse: replace module/path => ./local/path
            if let Some(pos) = rest.find("=>") {
                let target = rest[pos + 2..].trim();
                if target.starts_with("./") {
                    local_replaces.push(target.to_string());
                }
            }
        }
    })?;

    module.map(|m| GoMod {
        module: m,
        go_version,
        local_replaces,
    })
}

// --- go.work parsing ---

struct GoWork {
    use_dirs: Vec<String>,
}

fn parse_go_work(project_root: &Path) -> Option<GoWork> {
    let mut use_dirs = Vec::new();
    let mut in_use_block = false;

    for_each_trimmed_line(&project_root.join("go.work"), |line| {
        if line == "use (" {
            in_use_block = true;
        } else if in_use_block {
            if line == ")" {
                in_use_block = false;
            } else if !line.is_empty() && !line.starts_with("//") {
                use_dirs.push(line.to_string());
            }
        } else if let Some(rest) = line.strip_prefix("use ") {
            // Single-line: `use ./mymod`
            let dir = rest.trim();
            if !dir.starts_with('(') {
                use_dirs.push(dir.to_string());
            }
        }
    })?;

    if use_dirs.is_empty() {
        None
    } else {
        Some(GoWork { use_dirs })
    }
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
    fn parse_go_mod_local_replaces() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module github.com/openbao/openbao\n\ngo 1.25.7\n\nreplace github.com/openbao/openbao/api/v2 => ./api\n\nreplace github.com/openbao/openbao/sdk/v2 => ./sdk\n",
        )
        .unwrap();
        let m = parse_go_mod(dir.path()).unwrap();
        assert_eq!(m.module, "github.com/openbao/openbao");
        assert_eq!(m.local_replaces, vec!["./api", "./sdk"]);
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

    // --- provider tests ---

    #[test]
    fn provider_name() {
        let provider = GoIdentityProvider;
        assert_eq!(provider.name(), "project_identity");
    }

    #[test]
    fn provider_about_fields_match_base() {
        let provider = GoIdentityProvider;
        let fields = provider.about_fields();
        let base = base_about_fields();
        assert_eq!(fields.len(), base.len());
        for (a, b) in fields.iter().zip(base.iter()) {
            assert_eq!(a.id, b.id);
        }
    }

    #[test]
    fn provide_simple_go_project() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module github.com/user/myapp\n\ngo 1.22\n",
        )
        .unwrap();

        let provider = GoIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        assert_eq!(id.name, "myapp");
        assert_eq!(id.stack_label, "Go");
        assert_eq!(id.stack_detail.as_deref(), Some("Go 1.22"));
        assert_eq!(id.module_label, "modules");
        assert!(id.module_count.is_none()); // single module, no replaces
    }

    #[test]
    fn provide_go_project_with_local_replaces() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module github.com/org/mono\n\ngo 1.21\n\nreplace github.com/org/mono/api => ./api\nreplace github.com/org/mono/sdk => ./sdk\n",
        )
        .unwrap();

        let provider = GoIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        assert_eq!(id.name, "mono");
        // 1 main module + 2 local replaces
        assert_eq!(id.module_count, Some(3));
    }

    #[test]
    fn provide_go_workspace_module_count_from_go_work() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module github.com/user/ws\n\ngo 1.21\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse (\n\t./svc-a\n\t./svc-b\n\t./lib\n)\n",
        )
        .unwrap();

        let provider = GoIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        // go.work takes precedence: 3 use dirs
        assert_eq!(id.module_count, Some(3));
    }

    #[test]
    fn provide_no_go_mod_falls_back_to_dir_name() {
        let dir = tempfile::tempdir().unwrap();

        let provider = GoIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        // Falls back to directory name
        let expected = dir
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert_eq!(id.name, expected);
        assert_eq!(id.stack_label, "Go");
        assert!(id.stack_detail.is_none());
        assert!(id.module_count.is_none());
    }

    #[test]
    fn provide_populates_repository_from_git_remote() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module github.com/openbao/openbao\n\ngo 1.21\n",
        )
        .unwrap();
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        std::fs::write(
            git_dir.join("config"),
            "[remote \"origin\"]\n\turl = https://github.com/openbao/openbao.git\n",
        )
        .unwrap();

        let provider = GoIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        assert_eq!(
            id.repository.as_deref(),
            Some("https://github.com/openbao/openbao")
        );
    }

    #[test]
    fn provide_no_git_leaves_repository_empty() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/foo\n\ngo 1.21\n",
        )
        .unwrap();

        let provider = GoIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        assert!(id.repository.is_none());
    }

    #[test]
    fn provide_simple_module_name() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.mod"), "module myutil\n\ngo 1.20\n").unwrap();

        let provider = GoIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        // No slashes, so name is the whole module path
        assert_eq!(id.name, "myutil");
    }

    // --- additional parse_go_mod tests ---

    #[test]
    fn parse_go_mod_ignores_remote_replaces() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/foo\n\ngo 1.21\n\nreplace example.com/bar => github.com/fork/bar v1.2.3\n",
        )
        .unwrap();
        let m = parse_go_mod(dir.path()).unwrap();
        assert!(m.local_replaces.is_empty());
    }

    #[test]
    fn parse_go_mod_whitespace_handling() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "  module   example.com/ws  \n\n  go   1.23  \n",
        )
        .unwrap();
        let m = parse_go_mod(dir.path()).unwrap();
        assert_eq!(m.module, "example.com/ws");
        assert_eq!(m.go_version, Some("1.23".to_string()));
    }

    #[test]
    fn parse_go_mod_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.mod"), "").unwrap();
        assert!(parse_go_mod(dir.path()).is_none());
    }

    #[test]
    fn parse_go_mod_no_module_line() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.mod"), "go 1.21\n").unwrap();
        assert!(parse_go_mod(dir.path()).is_none());
    }

    // --- additional parse_go_work tests ---

    #[test]
    fn parse_go_work_empty_use_block() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.work"), "go 1.21\n\nuse (\n)\n").unwrap();
        assert!(parse_go_work(dir.path()).is_none());
    }

    #[test]
    fn parse_go_work_comments_in_use_block() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse (\n\t// a comment\n\t./real\n\t// another\n)\n",
        )
        .unwrap();
        let w = parse_go_work(dir.path()).unwrap();
        assert_eq!(w.use_dirs, vec!["./real"]);
    }

    #[test]
    fn parse_go_work_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.work"), "").unwrap();
        assert!(parse_go_work(dir.path()).is_none());
    }

    #[test]
    fn parse_go_work_blank_lines_in_use_block() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse (\n\n\t./a\n\n\t./b\n\n)\n",
        )
        .unwrap();
        let w = parse_go_work(dir.path()).unwrap();
        assert_eq!(w.use_dirs, vec!["./a", "./b"]);
    }

    #[test]
    fn parse_go_work_multiple_single_line_uses() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\nuse ./first\nuse ./second\n",
        )
        .unwrap();
        let w = parse_go_work(dir.path()).unwrap();
        assert_eq!(w.use_dirs, vec!["./first", "./second"]);
    }
}
