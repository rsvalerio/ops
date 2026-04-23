//! Node.js stack `project_identity` + `project_units` providers.
//!
//! Parses `package.json` for name, version, description, license, authors,
//! homepage, repository, and engine. npm/yarn workspaces come from the
//! `workspaces` field; pnpm workspaces come from `pnpm-workspace.yaml`.

mod units;

use std::path::Path;

use ops_core::project_identity::{base_about_fields, AboutFieldDef, ProjectIdentity};
use ops_core::text::dir_name;
use ops_extension::{Context, DataProvider, DataProviderError, ExtensionType};
use serde::Deserialize;

const NAME: &str = "about-node";
const DESCRIPTION: &str = "Node project identity";
const SHORTNAME: &str = "about-node";
const DATA_PROVIDER_NAME: &str = "project_identity";

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
        let insert_pos = fields
            .iter()
            .position(|f| f.id == "coverage")
            .unwrap_or(fields.len());
        fields.insert(
            insert_pos,
            AboutFieldDef {
                id: "homepage",
                label: "Homepage",
                description: "Project homepage URL",
            },
        );
        fields
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let cwd = ctx.working_directory.clone();
        let parsed = parse_package_json(&cwd);

        let name = parsed
            .as_ref()
            .and_then(|p| p.name.clone())
            .unwrap_or_else(|| dir_name(&cwd).to_string());
        let version = parsed.as_ref().and_then(|p| p.version.clone());
        let description = parsed.as_ref().and_then(|p| p.description.clone());
        let license = parsed.as_ref().and_then(|p| p.license.clone());
        let homepage = parsed.as_ref().and_then(|p| p.homepage.clone());
        let authors = parsed
            .as_ref()
            .map(|p| p.authors.clone())
            .unwrap_or_default();
        let repository = parsed
            .as_ref()
            .and_then(|p| p.repository.clone())
            .or_else(|| ops_git::GitInfo::collect(&cwd).remote_url);

        let pkg_manager = detect_package_manager(&cwd, parsed.as_ref());
        let engine_node = parsed.as_ref().and_then(|p| p.engines_node.clone());
        let stack_detail = match (engine_node, pkg_manager) {
            (Some(v), Some(pm)) => Some(format!("Node {v} · {pm}")),
            (Some(v), None) => Some(format!("Node {v}")),
            (None, Some(pm)) => Some(pm.to_string()),
            (None, None) => None,
        };

        let mut identity =
            ProjectIdentity::new(name, "Node", cwd.display().to_string(), "packages");
        identity.version = version;
        identity.description = description;
        identity.stack_detail = stack_detail;
        identity.license = license;
        identity.authors = authors;
        identity.repository = repository;
        identity.homepage = homepage;

        serde_json::to_value(&identity).map_err(DataProviderError::from)
    }
}

// --- package.json parsing ---

#[derive(Debug, Default)]
struct PackageJson {
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    license: Option<String>,
    homepage: Option<String>,
    repository: Option<String>,
    authors: Vec<String>,
    engines_node: Option<String>,
    has_packagemanager: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawPackage {
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    license: Option<LicenseField>,
    homepage: Option<String>,
    repository: Option<RepositoryField>,
    author: Option<PersonField>,
    #[serde(default)]
    contributors: Vec<PersonField>,
    engines: Option<Engines>,
    #[serde(rename = "packageManager")]
    package_manager: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum LicenseField {
    Text(String),
    Object { r#type: Option<String> },
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RepositoryField {
    Text(String),
    Object { url: Option<String> },
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PersonField {
    Text(String),
    Object {
        name: Option<String>,
        email: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
struct Engines {
    node: Option<String>,
}

fn parse_package_json(project_root: &Path) -> Option<PackageJson> {
    let content = std::fs::read_to_string(project_root.join("package.json")).ok()?;
    let raw: RawPackage = serde_json::from_str(&content).ok()?;

    let mut out = PackageJson {
        name: raw.name,
        version: raw.version,
        description: raw
            .description
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        license: raw.license.and_then(|l| match l {
            LicenseField::Text(s) => Some(s),
            LicenseField::Object { r#type } => r#type,
        }),
        homepage: raw.homepage.filter(|s| !s.is_empty()),
        repository: raw.repository.and_then(|r| match r {
            RepositoryField::Text(s) => Some(normalize_repo_url(&s)),
            RepositoryField::Object { url } => url.map(|u| normalize_repo_url(&u)),
        }),
        engines_node: raw.engines.and_then(|e| e.node),
        has_packagemanager: raw.package_manager,
        ..PackageJson::default()
    };

    let mut authors = Vec::new();
    if let Some(a) = raw.author {
        if let Some(s) = format_person(a) {
            authors.push(s);
        }
    }
    for c in raw.contributors {
        if let Some(s) = format_person(c) {
            authors.push(s);
        }
    }
    out.authors = authors;

    Some(out)
}

fn format_person(p: PersonField) -> Option<String> {
    match p {
        PersonField::Text(s) => Some(s).filter(|s| !s.is_empty()),
        PersonField::Object { name, email } => match (name, email) {
            (Some(n), Some(e)) => Some(format!("{n} <{e}>")),
            (Some(n), None) => Some(n),
            (None, Some(e)) => Some(e),
            (None, None) => None,
        },
    }
}

/// Normalize shorthand repository URLs used by npm:
/// - `github:user/repo` → `https://github.com/user/repo`
/// - `git+https://…` / `git://…` → stripped scheme
fn normalize_repo_url(raw: &str) -> String {
    let s = raw.trim();
    if let Some(rest) = s.strip_prefix("github:") {
        return format!("https://github.com/{rest}");
    }
    if let Some(rest) = s.strip_prefix("gitlab:") {
        return format!("https://gitlab.com/{rest}");
    }
    if let Some(rest) = s.strip_prefix("bitbucket:") {
        return format!("https://bitbucket.org/{rest}");
    }
    if let Some(rest) = s.strip_prefix("git+") {
        return rest.trim_end_matches(".git").to_string();
    }
    if let Some(rest) = s.strip_prefix("git://") {
        return format!("https://{}", rest.trim_end_matches(".git"));
    }
    s.trim_end_matches(".git").to_string()
}

fn detect_package_manager(
    project_root: &Path,
    parsed: Option<&PackageJson>,
) -> Option<&'static str> {
    // `packageManager` field takes precedence.
    if let Some(pm) = parsed.and_then(|p| p.has_packagemanager.as_deref()) {
        let name = pm.split('@').next().unwrap_or(pm);
        return match name {
            "pnpm" => Some("pnpm"),
            "yarn" => Some("yarn"),
            "npm" => Some("npm"),
            _ => None,
        };
    }
    if project_root.join("pnpm-lock.yaml").exists()
        || project_root.join("pnpm-workspace.yaml").exists()
    {
        return Some("pnpm");
    }
    if project_root.join("yarn.lock").exists() {
        return Some("yarn");
    }
    if project_root.join("bun.lockb").exists() || project_root.join("bun.lock").exists() {
        return Some("bun");
    }
    if project_root.join("package-lock.json").exists() {
        return Some("npm");
    }
    None
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
        // Conflicting lockfile — field wins.
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
