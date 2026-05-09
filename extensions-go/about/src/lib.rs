//! Go stack `project_identity` + `project_units` providers.
//!
//! Parses `go.mod` for module name, Go version, and local `replace` directives.
//! Parses `go.work` for workspace modules.
//!
//! Parse and read errors fall back to defaults; non-NotFound read errors and
//! parse errors are reported via `tracing` (`debug!` / `warn!`) so a malformed
//! manifest does not silently look like a missing one (TASK-0394).

mod go_mod;
mod go_syntax;
mod go_work;
mod modules;

use std::path::Path;

use ops_about::identity::{provide_identity_from_manifest, ParsedManifest};
use ops_core::project_identity::{base_about_fields, AboutFieldDef};
use ops_extension::{Context, DataProvider, DataProviderError, ExtensionType};

const NAME: &str = "about-go";
const DESCRIPTION: &str = "Go project identity";
const SHORTNAME: &str = "about-go";
const DATA_PROVIDER_NAME: &str = "project_identity";

#[non_exhaustive]
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
        provide_identity_from_manifest(ctx.working_directory.as_path(), |root| {
            let go_mod = parse_go_mod(root);
            // DUP-1 (TASK-0484): GoWork was a single-field newtype with no
            // semantic value. Use the parsed `Vec<String>` directly.
            let go_work_use_dirs = go_work::parse_use_dirs(root);

            // Use last segment of module path as name,
            // e.g. "github.com/openbao/openbao" → "openbao"
            let name = go_mod
                .as_ref()
                .and_then(|m| m.module.rsplit('/').next())
                .map(str::to_string);

            let stack_detail = go_mod
                .as_ref()
                .and_then(|m| m.go_version.clone())
                .map(|v| format!("Go {v}"));

            let module_count = compute_module_count(go_work_use_dirs.as_deref(), go_mod.as_ref());

            ParsedManifest::build(|m| {
                m.name = name;
                m.stack_label = "Go";
                m.stack_detail = stack_detail;
                m.module_label = "modules";
                m.module_count = module_count;
            })
        })
    }
}

/// Compute the module count surfaced in the About card.
///
/// Precedence: `go.work` use-dir count, else `go.mod` (1 + local replaces),
/// else `None`. A bare `go.mod` with no local replaces returns `None` so the
/// card omits a meaningless `1`.
fn compute_module_count(
    go_work_use_dirs: Option<&[String]>,
    go_mod: Option<&GoMod>,
) -> Option<usize> {
    if let Some(use_dirs) = go_work_use_dirs {
        return Some(use_dirs.len());
    }
    let m = go_mod?;
    let has_local_replaces = !m.local_replaces.is_empty();
    has_local_replaces.then(|| 1 + m.local_replaces.len())
}

// --- go.mod parsing ---

struct GoMod {
    module: String,
    go_version: Option<String>,
    local_replaces: Vec<String>,
}

fn parse_go_mod(project_root: &Path) -> Option<GoMod> {
    let raw = go_mod::parse(project_root)?;
    raw.module.map(|m| GoMod {
        module: m,
        go_version: raw.go_version,
        local_replaces: raw.local_replaces,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_core::project_identity::ProjectIdentity;

    #[test]
    fn compute_module_count_workspace_precedence() {
        let work = vec!["./a".to_string(), "./b".to_string()];
        let m = GoMod {
            module: "x".to_string(),
            go_version: None,
            local_replaces: vec!["./c".to_string()],
        };
        // go.work wins — even when go.mod has replaces.
        assert_eq!(compute_module_count(Some(&work), Some(&m)), Some(2));
    }

    #[test]
    fn compute_module_count_single_module_returns_none() {
        let m = GoMod {
            module: "x".to_string(),
            go_version: None,
            local_replaces: Vec::new(),
        };
        assert_eq!(compute_module_count(None, Some(&m)), None);
    }

    #[test]
    fn compute_module_count_with_local_replaces() {
        let m = GoMod {
            module: "x".to_string(),
            go_version: None,
            local_replaces: vec!["./a".to_string(), "./b".to_string()],
        };
        assert_eq!(compute_module_count(None, Some(&m)), Some(3));
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
}
