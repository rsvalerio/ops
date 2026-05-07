//! Python stack `project_identity` + `project_units` providers.
//!
//! Parses `pyproject.toml` (PEP 621) for name, version, description, license,
//! authors, Python requirement, homepage, and repository. Detects uv by the
//! presence of `uv.lock` or `[tool.uv]` and surfaces it in the stack detail.
//! Workspace members come from `[tool.uv.workspace].members`.
//!
//! Parse and read errors fall back to defaults; non-NotFound read errors and
//! parse errors are reported via `tracing` (`debug!` / `warn!`) so a malformed
//! manifest does not silently look like a missing one (TASK-0394).

mod manifest_cache;
mod units;

use std::path::Path;

use ops_about::identity::{provide_identity_from_manifest, ParsedManifest};
use ops_core::project_identity::{base_about_fields, insert_homepage_field, AboutFieldDef};
use ops_extension::{Context, DataProvider, DataProviderError, ExtensionType};
use serde::Deserialize;

const NAME: &str = "about-python";
const DESCRIPTION: &str = "Python project identity";
const SHORTNAME: &str = "about-python";
const DATA_PROVIDER_NAME: &str = "project_identity";

#[non_exhaustive]
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
        registry.register(units::PROVIDER_NAME, Box::new(units::PythonUnitsProvider));
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
        insert_homepage_field(&mut fields);
        fields
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        // DUP-1 (TASK-0484): proof-of-concept of `provide_identity_from_manifest`
        // — the parse-once / build-identity scaffold lives in `ops_about`,
        // and the Python provider only needs to project pyproject.toml onto
        // a [`ParsedManifest`].
        provide_identity_from_manifest(ctx.working_directory.as_path(), |root| {
            let Pyproject {
                name,
                version,
                description,
                license,
                requires_python,
                authors,
                homepage,
                repository,
                has_tool_uv,
            } = parse_pyproject(root).unwrap_or_default();

            // SEC-25 (mirrors extensions-node/about/src/package_manager.rs::probe):
            // use symlink_metadata so a hostile uv.lock symlink isn't followed
            // to an arbitrary target during workspace probing.
            let uses_uv = std::fs::symlink_metadata(root.join("uv.lock")).is_ok() || has_tool_uv;
            let stack_detail = build_stack_detail(requires_python.as_deref(), uses_uv);

            ParsedManifest::build(|m| {
                m.name = name;
                m.version = version;
                m.description = description;
                m.license = license;
                m.authors = authors;
                m.homepage = homepage;
                m.repository = repository;
                m.stack_label = "Python";
                m.stack_detail = stack_detail;
                m.module_label = "packages";
                m.module_count = None;
            })
        })
    }
}

/// Compose the `stack_detail` string from optional `requires-python` value
/// and a boolean indicating whether uv is in use.
fn build_stack_detail(requires_python: Option<&str>, uses_uv: bool) -> Option<String> {
    match (requires_python, uses_uv) {
        (Some(v), true) => Some(format!("Python {v} · uv")),
        (Some(v), false) => Some(format!("Python {v}")),
        (None, true) => Some("uv".to_string()),
        (None, false) => None,
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
    // PERF-3 / TASK-0569: only presence of `[tool.uv]` matters here. Using
    // `serde::de::IgnoredAny` skips the entire subtree (often holding
    // dev-dependencies, sources, indexes) instead of materialising it into
    // an arbitrary `toml::Value` that is immediately thrown away.
    uv: Option<serde::de::IgnoredAny>,
}

fn parse_pyproject(project_root: &Path) -> Option<Pyproject> {
    // DUP-3 / TASK-0816: read+parse pyproject.toml at most once per project
    // root for the lifetime of the process; the units provider deserialises
    // its own shape from the same shared `toml::Value`.
    // PERF-3 / TASK-0854: read directly from the cached raw text and let
    // toml::from_str project straight into RawPyproject — avoids the prior
    // `(*value).clone().try_into()` which materialised a fresh 2-10 KB
    // toml::Value tree per provider call.
    let text = manifest_cache::pyproject_text(project_root)?;
    let raw: RawPyproject = match toml::from_str(&text) {
        Ok(r) => r,
        Err(e) => {
            // ERR-7 / TASK-0974: include the manifest path so multi-root
            // `ops about` runs can attribute the parse failure. Debug-format
            // the path so embedded newlines / ANSI in attacker-controlled
            // checkout paths cannot forge log lines.
            tracing::warn!(
                path = ?project_root.join("pyproject.toml").display(),
                error = %e,
                recovery = "default-identity",
                "failed to project pyproject.toml into identity shape"
            );
            return None;
        }
    };

    let mut out = Pyproject {
        has_tool_uv: raw.tool.as_ref().and_then(|t| t.uv.as_ref()).is_some(),
        ..Pyproject::default()
    };

    if let Some(p) = raw.project {
        out.name = trim_nonempty(p.name);
        out.version = trim_nonempty(p.version);
        out.description = trim_nonempty(p.description);
        out.requires_python = trim_nonempty(p.requires_python);
        out.license = p.license.and_then(normalize_license);
        out.authors = format_authors(p.authors.unwrap_or_default());
        if let Some(urls) = p.urls {
            let (homepage, repository) = extract_urls(&urls);
            out.homepage = homepage;
            out.repository = repository;
        }
    }

    Some(out)
}

/// PEP 621 license can be a string, `{ text = "..." }`, or `{ file = "LICENSE" }`.
/// The file form is a *path* to a file, not an SPDX identifier, so passing it
/// through as the license name is misleading. When only `file` is set, surface
/// it explicitly as `License file: <name>` so the About card communicates that
/// an SPDX identifier was not declared but a license file is present.
/// ERR-2 / TASK-0704: trim+drop-empty for license text so a whitespace-only
/// field does not render as a blank bullet.
fn normalize_license(license: LicenseField) -> Option<String> {
    match license {
        LicenseField::Text(s) => trim_nonempty(Some(s)),
        LicenseField::Table { text: Some(t), .. } => trim_nonempty(Some(t)),
        LicenseField::Table { file: Some(f), .. } => {
            trim_nonempty(Some(f)).map(|f| format!("License file: {f}"))
        }
        LicenseField::Table { .. } => None,
    }
}

/// ERR-2 / TASK-0704: trim+drop-empty for each author component so a
/// whitespace-only field does not render as a blank bullet — matching
/// package.json's `format_person`.
fn format_authors(authors: Vec<RawAuthor>) -> Vec<String> {
    authors
        .into_iter()
        .filter_map(|a| {
            let name = trim_nonempty(a.name);
            let email = trim_nonempty(a.email);
            match (name, email) {
                (Some(n), Some(e)) => Some(format!("{n} <{e}>")),
                (Some(n), None) => Some(n),
                // ERR-2 / TASK-0980: render the email-only case as
                // `<email>` to match `extensions-node/about::format_person`
                // — both providers feed the same About card schema and a
                // bare email next to "Name <email>" entries renders
                // inconsistently in a multi-author list.
                (None, Some(e)) => Some(format!("<{e}>")),
                (None, None) => None,
            }
        })
        .collect()
}

fn extract_urls(
    urls: &std::collections::BTreeMap<String, String>,
) -> (Option<String>, Option<String>) {
    // PERF-3 / TASK-0991: normalise each URL key exactly once per About
    // call. Previously `pick_url` built a fresh `Vec<(String, &String)>`
    // and re-ran `normalize_url_key` over every key on each invocation;
    // `extract_urls` calls `pick_url` twice, so the work was duplicated.
    let normalized = normalize_urls(urls);
    // PATTERN-1 / TASK-1062: PEP 621 distinguishes `Homepage` from
    // `Documentation` as separate, semantically distinct labels. Folding
    // `documentation` into the homepage slot misrepresents a docs-only
    // pyproject as having its docs URL as the homepage, and silently
    // discards Documentation when both are present. Drop it from the
    // homepage candidate list so its absence falls through to None. If the
    // About card grows a Documentation field, surface it as its own bullet.
    let homepage = pick_url(&normalized, &["homepage", "home", "home-page"]);
    let repository = pick_url(
        &normalized,
        &[
            "repository",
            "source",
            "source-code",
            "sourcecode",
            "code",
            "repo",
        ],
    );
    (homepage, repository)
}

/// PEP 621 places no constraints on `[project.urls]` key casing or spelling
/// (`Homepage`, `homepage`, `Home Page`, `home-page` are all common in the
/// wild). Look up candidates case-insensitively after trimming, and accept the
/// kebab-case variant as equivalent to the space-separated form. Callers should
/// pass the canonical kebab/space form for each variant — "home-page" and
/// "home page" normalise identically, so passing both is dead weight.
/// PERF-3 / TASK-0991: shared normalisation pass — once per About call,
/// rather than once per pick_url candidate-set.
fn normalize_urls(
    urls: &std::collections::BTreeMap<String, String>,
) -> std::collections::HashMap<String, &String> {
    urls.iter()
        .map(|(k, v)| (normalize_url_key(k), v))
        .collect()
}

fn pick_url(
    normalized: &std::collections::HashMap<String, &String>,
    keys: &[&str],
) -> Option<String> {
    keys.iter()
        .find_map(|target| {
            let target_norm = normalize_url_key(target);
            normalized.get(&target_norm).map(|v| (*v).clone())
        })
        // TASK-0964: align with the workspace-wide ERR-2 / TASK-0704 trim+drop
        // policy so a whitespace-only URL renders as "no homepage" instead of
        // an empty About bullet.
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn normalize_url_key(key: &str) -> String {
    key.trim().to_ascii_lowercase().replace('-', " ")
}

fn trim_nonempty(value: Option<String>) -> Option<String> {
    value
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_core::project_identity::ProjectIdentity;

    /// ERR-7 (TASK-0818): manifest paths flow through `tracing::warn!` via
    /// the `?` formatter so embedded newlines or ANSI escapes cannot forge
    /// multi-line log records. DUP-3 / TASK-0985: shared helper — see
    /// `ops_about::test_support`.
    #[test]
    fn pyproject_path_debug_escapes_control_characters() {
        let p = Path::new("a\nb\u{1b}[31mc/pyproject.toml");
        ops_about::test_support::assert_debug_escapes_control_chars(p.display());
    }

    #[test]
    fn build_stack_detail_python_with_uv() {
        assert_eq!(
            build_stack_detail(Some(">=3.11"), true),
            Some("Python >=3.11 · uv".to_string())
        );
    }

    #[test]
    fn build_stack_detail_python_only() {
        assert_eq!(
            build_stack_detail(Some(">=3.11"), false),
            Some("Python >=3.11".to_string())
        );
    }

    #[test]
    fn build_stack_detail_uv_only() {
        assert_eq!(build_stack_detail(None, true), Some("uv".to_string()));
    }

    #[test]
    fn build_stack_detail_neither() {
        assert_eq!(build_stack_detail(None, false), None);
    }

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
    fn whitespace_only_license_and_author_components_are_dropped() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            r#"
[project]
name = "demo"
version = "0.1.0"
license = "  "
authors = [{ name = "  ", email = "  " }]
"#,
        )
        .unwrap();

        let provider = PythonIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();

        assert!(id.license.is_none());
        assert!(id.authors.is_empty());
    }

    #[test]
    fn whitespace_only_requires_python_does_not_render() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\nname = \"demo\"\nversion = \"0.1.0\"\nrequires-python = \"  \"\n",
        )
        .unwrap();

        let provider = PythonIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();

        assert_eq!(id.stack_detail, None);
    }

    #[test]
    fn pick_url_repository_takes_precedence_over_source_and_source_code() {
        let mut urls = std::collections::BTreeMap::new();
        urls.insert(
            "Source-Code".to_string(),
            "https://example.com/sc".to_string(),
        );
        urls.insert("source".to_string(), "https://example.com/src".to_string());
        urls.insert(
            "Repository".to_string(),
            "https://example.com/repo".to_string(),
        );

        let picked = pick_url(
            &normalize_urls(&urls),
            &[
                "repository",
                "source",
                "source-code",
                "sourcecode",
                "code",
                "repo",
            ],
        );
        assert_eq!(picked.as_deref(), Some("https://example.com/repo"));

        urls.remove("Repository");
        let picked = pick_url(
            &normalize_urls(&urls),
            &[
                "repository",
                "source",
                "source-code",
                "sourcecode",
                "code",
                "repo",
            ],
        );
        assert_eq!(picked.as_deref(), Some("https://example.com/src"));

        urls.remove("source");
        let picked = pick_url(
            &normalize_urls(&urls),
            &[
                "repository",
                "source",
                "source-code",
                "sourcecode",
                "code",
                "repo",
            ],
        );
        assert_eq!(picked.as_deref(), Some("https://example.com/sc"));
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
    fn license_file_form_is_labeled() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            r#"
[project]
name = "demo"
version = "1.0.0"
license = { file = "LICENSE" }
"#,
        )
        .unwrap();

        let provider = PythonIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();

        assert_eq!(id.license.as_deref(), Some("License file: LICENSE"));
    }

    #[test]
    fn parses_urls_case_insensitive_and_kebab() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            r#"
[project]
name = "demo"
version = "1.0.0"

[project.urls]
homepage = "https://demo.dev"
source-code = "https://github.com/x/demo"
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

    /// TASK-0964: a whitespace-only URL must drop to None instead of rendering
    /// as an empty About bullet, matching the trim+drop policy already applied
    /// to name/license/requires-python/authors.
    #[test]
    fn whitespace_only_url_resolves_to_none() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            "[project]\n\
             name = \"demo\"\n\
             version = \"1.0.0\"\n\
             \n\
             [project.urls]\n\
             Homepage = \"   \"\n",
        )
        .unwrap();

        let provider = PythonIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();

        assert!(
            id.homepage.is_none(),
            "whitespace-only Homepage must drop, got: {:?}",
            id.homepage
        );
    }

    /// PATTERN-1 / TASK-1062: PEP 621 distinguishes `Homepage` from
    /// `Documentation`. A pyproject with only a `Documentation` URL must NOT
    /// have its docs URL surfaced as the project homepage — the homepage
    /// field should fall through to None.
    #[test]
    fn documentation_only_url_does_not_become_homepage() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            r#"
[project]
name = "demo"
version = "1.0.0"

[project.urls]
Documentation = "https://docs.x"
"#,
        )
        .unwrap();

        let provider = PythonIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();

        assert!(
            id.homepage.is_none(),
            "Documentation must not be folded into homepage, got: {:?}",
            id.homepage
        );
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

    /// ERR-2 / TASK-0980: an email-only author renders as `<email>` so
    /// the python provider matches `extensions-node` `format_person`.
    /// Without the brackets, a bare email next to "Name <email>" entries
    /// renders ambiguously in a multi-author card.
    #[test]
    fn email_only_author_renders_with_angle_brackets() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pyproject.toml"),
            r#"
[project]
name = "demo"
version = "0.1.0"
authors = [
    { email = "a@example.com" },
]
"#,
        )
        .unwrap();

        let provider = PythonIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();

        assert_eq!(id.authors, vec!["<a@example.com>".to_string()]);
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
