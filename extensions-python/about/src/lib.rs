//! Python stack `project_identity` provider.
//!
//! Parses `pyproject.toml` (PEP 621) for name, version, description, license,
//! authors, Python requirement, homepage, and repository. Detects uv by the
//! presence of `uv.lock` or `[tool.uv]` and surfaces it in the stack detail.

use std::path::Path;

use ops_core::project_identity::{base_about_fields, AboutFieldDef, ProjectIdentity};
use ops_core::text::dir_name;
use ops_extension::{Context, DataProvider, DataProviderError, ExtensionType};
use serde::Deserialize;

const NAME: &str = "about-python";
const DESCRIPTION: &str = "Python project identity";
const SHORTNAME: &str = "about-python";
const DATA_PROVIDER_NAME: &str = "project_identity";

pub struct AboutPythonExtension;

ops_extension::impl_extension! {
    AboutPythonExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE,
    stack: Some(ops_extension::Stack::Python),
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_data_providers: |_self, registry| {
        registry.register(DATA_PROVIDER_NAME, Box::new(PythonIdentityProvider));
    },
    factory: PYTHON_ABOUT_FACTORY = |_, _| {
        Some((NAME, Box::new(AboutPythonExtension)))
    },
}

struct PythonIdentityProvider;

impl DataProvider for PythonIdentityProvider {
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
        let parsed = parse_pyproject(&cwd);

        let name = parsed
            .as_ref()
            .and_then(|p| p.name.clone())
            .unwrap_or_else(|| dir_name(&cwd).to_string());
        let version = parsed.as_ref().and_then(|p| p.version.clone());
        let description = parsed.as_ref().and_then(|p| p.description.clone());
        let license = parsed.as_ref().and_then(|p| p.license.clone());
        let requires_python = parsed.as_ref().and_then(|p| p.requires_python.clone());
        let homepage = parsed.as_ref().and_then(|p| p.homepage.clone());
        let authors = parsed
            .as_ref()
            .map(|p| p.authors.clone())
            .unwrap_or_default();
        let repository = parsed
            .as_ref()
            .and_then(|p| p.repository.clone())
            .or_else(|| ops_git::GitInfo::collect(&cwd).remote_url);

        let uses_uv = detect_uv(&cwd, parsed.as_ref());
        let stack_detail = match (requires_python, uses_uv) {
            (Some(v), true) => Some(format!("Python {v} · uv")),
            (Some(v), false) => Some(format!("Python {v}")),
            (None, true) => Some("uv".to_string()),
            (None, false) => None,
        };

        let identity = ProjectIdentity {
            name,
            version,
            description,
            stack_label: "Python".to_string(),
            stack_detail,
            license,
            project_path: cwd.display().to_string(),
            module_label: "packages".to_string(),
            authors,
            repository,
            homepage,
            ..Default::default()
        };

        serde_json::to_value(&identity).map_err(DataProviderError::from)
    }
}

// --- pyproject.toml parsing (PEP 621) ---

#[derive(Debug, Default)]
struct Pyproject {
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    license: Option<String>,
    requires_python: Option<String>,
    authors: Vec<String>,
    homepage: Option<String>,
    repository: Option<String>,
    has_tool_uv: bool,
}

#[derive(Debug, Deserialize)]
struct RawPyproject {
    project: Option<RawProject>,
    tool: Option<RawTool>,
}

#[derive(Debug, Deserialize)]
struct RawProject {
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    #[serde(rename = "requires-python")]
    requires_python: Option<String>,
    license: Option<LicenseField>,
    authors: Option<Vec<RawAuthor>>,
    urls: Option<std::collections::BTreeMap<String, String>>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum LicenseField {
    Text(String),
    Table {
        text: Option<String>,
        file: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
struct RawAuthor {
    name: Option<String>,
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawTool {
    uv: Option<toml::Value>,
}

fn parse_pyproject(project_root: &Path) -> Option<Pyproject> {
    let content = std::fs::read_to_string(project_root.join("pyproject.toml")).ok()?;
    let raw: RawPyproject = toml::from_str(&content).ok()?;

    let mut out = Pyproject {
        has_tool_uv: raw.tool.as_ref().and_then(|t| t.uv.as_ref()).is_some(),
        ..Pyproject::default()
    };

    if let Some(p) = raw.project {
        out.name = p.name;
        out.version = p.version;
        out.description = p
            .description
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        out.requires_python = p.requires_python;
        out.license = p.license.and_then(|l| match l {
            LicenseField::Text(s) => Some(s),
            LicenseField::Table { text, file } => text.or(file),
        });
        out.authors = p
            .authors
            .unwrap_or_default()
            .into_iter()
            .filter_map(|a| match (a.name, a.email) {
                (Some(n), Some(e)) => Some(format!("{n} <{e}>")),
                (Some(n), None) => Some(n),
                (None, Some(e)) => Some(e),
                (None, None) => None,
            })
            .collect();
        if let Some(urls) = p.urls {
            out.homepage = pick_url(&urls, &["Homepage", "homepage", "Home", "Documentation"]);
            out.repository = pick_url(
                &urls,
                &[
                    "Repository",
                    "repository",
                    "Source",
                    "source",
                    "Source Code",
                ],
            );
        }
    }

    Some(out)
}

fn pick_url(urls: &std::collections::BTreeMap<String, String>, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|k| urls.get(*k).cloned())
        .filter(|s| !s.is_empty())
}

fn detect_uv(project_root: &Path, parsed: Option<&Pyproject>) -> bool {
    if project_root.join("uv.lock").exists() {
        return true;
    }
    parsed.map(|p| p.has_tool_uv).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_name() {
        assert_eq!(PythonIdentityProvider.name(), "project_identity");
    }

    #[test]
    fn about_fields_include_homepage() {
        let fields = PythonIdentityProvider.about_fields();
        let ids: Vec<&str> = fields.iter().map(|f| f.id).collect();
        assert!(ids.contains(&"homepage"));
    }

    #[test]
    fn parse_minimal_pyproject() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            r#"
[project]
name = "codeagent-bench"
version = "0.0.1"
description = "Benchmark harness for mix-and-match {coding-agent} x {MCP} x {prompts} evaluation"
readme = "README.md"
requires-python = ">=3.11"
license = { text = "MIT" }
authors = [{ name = "rsvaleri" }]
"#,
        )
        .unwrap();

        let provider = PythonIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();

        assert_eq!(id.name, "codeagent-bench");
        assert_eq!(id.version.as_deref(), Some("0.0.1"));
        assert_eq!(id.stack_label, "Python");
        assert_eq!(id.stack_detail.as_deref(), Some("Python >=3.11"));
        assert_eq!(id.license.as_deref(), Some("MIT"));
        assert_eq!(id.authors, vec!["rsvaleri"]);
        assert_eq!(id.module_label, "packages");
    }

    #[test]
    fn detects_uv_from_lockfile() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\nname = \"demo\"\nversion = \"0.1.0\"\nrequires-python = \">=3.12\"\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("uv.lock"), "# uv lockfile\n").unwrap();

        let provider = PythonIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();

        assert_eq!(id.stack_detail.as_deref(), Some("Python >=3.12 · uv"));
    }

    #[test]
    fn detects_uv_from_tool_table() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            r#"
[project]
name = "demo"
version = "0.1.0"

[tool.uv]
dev-dependencies = []
"#,
        )
        .unwrap();

        let provider = PythonIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();

        assert_eq!(id.stack_detail.as_deref(), Some("uv"));
    }

    #[test]
    fn parses_urls_homepage_and_repository() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            r#"
[project]
name = "demo"
version = "1.0.0"

[project.urls]
Homepage = "https://demo.dev"
Repository = "https://github.com/x/demo"
"#,
        )
        .unwrap();

        let provider = PythonIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();

        assert_eq!(id.homepage.as_deref(), Some("https://demo.dev"));
        assert_eq!(id.repository.as_deref(), Some("https://github.com/x/demo"));
    }

    #[test]
    fn fallback_to_dir_name_when_no_pyproject() {
        let dir = tempfile::tempdir().unwrap();
        let provider = PythonIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();

        assert_eq!(id.stack_label, "Python");
        assert!(id.version.is_none());
        assert!(id.stack_detail.is_none());
    }

    #[test]
    fn author_with_name_and_email() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            r#"
[project]
name = "demo"
version = "0.1.0"
authors = [
    { name = "Alice", email = "a@example.com" },
    { name = "Bob" },
]
"#,
        )
        .unwrap();

        let provider = PythonIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();

        assert_eq!(id.authors, vec!["Alice <a@example.com>", "Bob"]);
    }

    #[test]
    fn git_remote_fallback_when_no_repository_url() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\nname = \"demo\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        std::fs::write(
            git_dir.join("config"),
            "[remote \"origin\"]\n\turl = https://github.com/o/r.git\n",
        )
        .unwrap();

        let provider = PythonIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();

        assert_eq!(id.repository.as_deref(), Some("https://github.com/o/r"));
    }
}
