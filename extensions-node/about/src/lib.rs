//! Node.js stack `project_identity` + `project_units` providers.
//!
//! Parses `package.json` for name, version, description, license, authors,
//! homepage, repository, and engine. npm/yarn workspaces come from the
//! `workspaces` field; pnpm workspaces come from `pnpm-workspace.yaml`.
//!
//! Parse and read errors fall back to defaults; non-NotFound read errors and
//! parse errors are reported via `tracing` (`debug!` / `warn!`) so a malformed
//! manifest does not silently look like a missing one (TASK-0394).

mod manifest_cache;
mod package_json;
mod package_manager;
mod repo_url;
mod units;

use ops_about::identity::{provide_identity_from_manifest, ParsedManifest};
use ops_core::project_identity::{base_about_fields, insert_homepage_field, AboutFieldDef};
use ops_extension::{Context, DataProvider, DataProviderError, ExtensionType};

use package_json::{parse_package_json, PackageJson};
use package_manager::detect_package_manager;

const NAME: &str = "about-node";
const DESCRIPTION: &str = "Node project identity";
const SHORTNAME: &str = "about-node";
const DATA_PROVIDER_NAME: &str = "project_identity";

#[non_exhaustive]
pub struct AboutNodeExtension;

ops_extension::impl_extension! {
    AboutNodeExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE,
    stack: Some(ops_extension::Stack::Node),
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_data_providers: |_self, registry| {
        registry.register(DATA_PROVIDER_NAME, Box::new(NodeIdentityProvider));
        registry.register(units::PROVIDER_NAME, Box::new(units::NodeUnitsProvider));
    },
    factory: NODE_ABOUT_FACTORY = |_, _| {
        Some((NAME, Box::new(AboutNodeExtension)))
    },
}

struct NodeIdentityProvider;

impl DataProvider for NodeIdentityProvider {
    fn name(&self) -> &'static str {
        DATA_PROVIDER_NAME
    }

    fn about_fields(&self) -> Vec<AboutFieldDef> {
        let mut fields = base_about_fields();
        insert_homepage_field(&mut fields);
        fields
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        provide_identity_from_manifest(ctx.working_directory.as_path(), |root| {
            let PackageJson {
                name,
                version,
                description,
                license,
                homepage,
                repository,
                authors,
                engines_node,
                has_packagemanager,
            } = parse_package_json(root).unwrap_or_default();

            let pkg_manager = detect_package_manager(root, has_packagemanager.as_deref());
            let stack_detail = build_stack_detail(engines_node.as_deref(), pkg_manager);

            ParsedManifest::build(|m| {
                m.name = name;
                m.version = version;
                m.description = description;
                m.license = license;
                m.authors = authors;
                m.homepage = homepage;
                m.repository = repository;
                m.stack_label = "Node";
                m.stack_detail = stack_detail;
                m.module_label = "packages";
                m.module_count = None;
            })
        })
    }
}

/// Compose the `stack_detail` string from optional Node engine version and
/// optional package-manager label.
fn build_stack_detail(engine_node: Option<&str>, pkg_manager: Option<&str>) -> Option<String> {
    match (engine_node, pkg_manager) {
        (Some(v), Some(pm)) => Some(format!("Node {v} · {pm}")),
        (Some(v), None) => Some(format!("Node {v}")),
        (None, Some(pm)) => Some(pm.to_string()),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_core::project_identity::ProjectIdentity;
    use std::path::Path;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn build_stack_detail_both_set() {
        assert_eq!(
            build_stack_detail(Some(">=18"), Some("pnpm")),
            Some("Node >=18 · pnpm".to_string())
        );
    }

    #[test]
    fn build_stack_detail_engine_only() {
        assert_eq!(
            build_stack_detail(Some(">=18"), None),
            Some("Node >=18".to_string())
        );
    }

    #[test]
    fn build_stack_detail_pm_only() {
        assert_eq!(
            build_stack_detail(None, Some("pnpm")),
            Some("pnpm".to_string())
        );
    }

    #[test]
    fn build_stack_detail_neither() {
        assert_eq!(build_stack_detail(None, None), None);
    }

    #[test]
    fn provider_name() {
        assert_eq!(NodeIdentityProvider.name(), "project_identity");
    }

    #[test]
    fn about_fields_include_homepage() {
        let fields = NodeIdentityProvider.about_fields();
        let ids: Vec<&str> = fields.iter().map(|f| f.id).collect();
        assert!(ids.contains(&"homepage"));
    }

    #[test]
    fn parse_minimal_package_json() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("package.json"),
            r#"{
  "name": "my-pkg",
  "version": "1.2.3",
  "description": "Demo package",
  "license": "MIT",
  "author": "Alice <a@example.com>",
  "homepage": "https://demo.dev",
  "repository": "github:user/repo"
}"#,
        );
        let provider = NodeIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();
        assert_eq!(id.name, "my-pkg");
        assert_eq!(id.version.as_deref(), Some("1.2.3"));
        assert_eq!(id.description.as_deref(), Some("Demo package"));
        assert_eq!(id.license.as_deref(), Some("MIT"));
        assert_eq!(id.stack_label, "Node");
        assert_eq!(id.module_label, "packages");
        assert_eq!(id.homepage.as_deref(), Some("https://demo.dev"));
        assert_eq!(
            id.repository.as_deref(),
            Some("https://github.com/user/repo")
        );
        assert_eq!(id.authors, vec!["Alice <a@example.com>"]);
    }

    #[test]
    fn parse_author_object_and_contributors() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("package.json"),
            r#"{
  "name": "x",
  "author": { "name": "Alice", "email": "a@example.com" },
  "contributors": [
    "Bob <b@example.com>",
    { "name": "Carol" }
  ]
}"#,
        );
        let provider = NodeIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();
        assert_eq!(
            id.authors,
            vec!["Alice <a@example.com>", "Bob <b@example.com>", "Carol"]
        );
    }

    #[test]
    fn parse_repository_object_with_git_plus_url() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("package.json"),
            r#"{
  "name": "x",
  "repository": { "type": "git", "url": "git+https://github.com/o/r.git" }
}"#,
        );
        let provider = NodeIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();
        assert_eq!(id.repository.as_deref(), Some("https://github.com/o/r"));
    }

    #[test]
    fn detects_pnpm_via_workspace_file() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("package.json"),
            r#"{ "name": "x", "engines": { "node": ">=18" } }"#,
        );
        write(
            &dir.path().join("pnpm-workspace.yaml"),
            "packages:\n  - 'packages/*'\n",
        );
        let provider = NodeIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();
        assert_eq!(id.stack_detail.as_deref(), Some("Node >=18 · pnpm"));
    }

    #[test]
    fn detects_yarn_via_lockfile() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("package.json"), r#"{ "name": "x" }"#);
        write(&dir.path().join("yarn.lock"), "# yarn\n");
        let provider = NodeIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();
        assert_eq!(id.stack_detail.as_deref(), Some("yarn"));
    }

    #[test]
    fn package_manager_field_takes_precedence() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("package.json"),
            r#"{ "name": "x", "packageManager": "pnpm@9.0.0" }"#,
        );
        write(&dir.path().join("yarn.lock"), "");
        let provider = NodeIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();
        assert_eq!(id.stack_detail.as_deref(), Some("pnpm"));
    }

    #[test]
    fn fallback_to_dir_name_when_no_package_json() {
        let dir = tempfile::tempdir().unwrap();
        let provider = NodeIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();
        assert_eq!(id.stack_label, "Node");
        assert!(id.version.is_none());
    }

    #[test]
    fn git_remote_fallback_when_no_repository_field() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("package.json"), r#"{ "name": "x" }"#);
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        std::fs::write(
            git_dir.join("config"),
            "[remote \"origin\"]\n\turl = https://github.com/o/r.git\n",
        )
        .unwrap();
        let provider = NodeIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();
        assert_eq!(id.repository.as_deref(), Some("https://github.com/o/r"));
    }

    #[test]
    fn license_object_form() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("package.json"),
            r#"{ "name": "x", "license": { "type": "Apache-2.0" } }"#,
        );
        let provider = NodeIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();
        assert_eq!(id.license.as_deref(), Some("Apache-2.0"));
    }
}
