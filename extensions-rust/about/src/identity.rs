//! Rust-specific `project_identity` data provider.
//!
//! Reads Cargo.toml directly and queries DuckDB for LOC stats to build a
//! [`ProjectIdentity`](ops_core::project_identity::ProjectIdentity)
//! with Rust-specific fields (crates, edition, etc.).

use std::path::Path;

use ops_cargo_toml::{CargoToml, CargoTomlProvider};
use ops_core::project_identity::{base_about_fields, AboutFieldDef, LanguageStat, ProjectIdentity};
use ops_core::text::dir_name;
use ops_extension::{Context, DataProvider, DataProviderError};

use crate::query::resolve_member_globs;

pub(crate) const PROVIDER_NAME: &str = "project_identity";

pub(crate) struct RustIdentityProvider;

/// Resolves a string field by trying `[package]` first, then falling back to `[workspace.package]`.
fn resolve_field(
    pkg: Option<&ops_cargo_toml::Package>,
    ws_pkg: Option<&ops_cargo_toml::WorkspacePackage>,
    pkg_getter: impl Fn(&ops_cargo_toml::Package) -> Option<&str>,
    ws_getter: impl Fn(&ops_cargo_toml::WorkspacePackage) -> Option<&str>,
) -> Option<String> {
    pkg.and_then(&pkg_getter)
        .or_else(|| ws_pkg.and_then(&ws_getter))
        .map(|s| s.to_string())
}

/// Resolved inheritable fields from `[package]` / `[workspace.package]`.
struct ResolvedFields {
    version: Option<String>,
    description: Option<String>,
    edition: Option<String>,
    license: Option<String>,
    repository: Option<String>,
    homepage: Option<String>,
    msrv: Option<String>,
    authors: Vec<String>,
}

/// Metrics queried from DuckDB (LOC, dependencies, coverage, languages).
struct IdentityMetrics {
    loc: Option<i64>,
    file_count: Option<i64>,
    dependency_count: Option<usize>,
    coverage_percent: Option<f64>,
    languages: Vec<LanguageStat>,
}

fn resolve_identity_fields(
    pkg: Option<&ops_cargo_toml::Package>,
    ws_pkg: Option<&ops_cargo_toml::WorkspacePackage>,
    cwd: &Path,
) -> ResolvedFields {
    // Resolve one inheritable string field. `$pg` names the `Package` field
    // (InheritableString — uses `.as_str()`), `$wg` names the corresponding
    // `WorkspacePackage` field (`Option<String>` — uses `.as_deref()`).
    macro_rules! r {
        ($pg:ident, $wg:ident) => {
            resolve_field(pkg, ws_pkg, |p| p.$pg.as_str(), |wp| wp.$wg.as_deref())
        };
    }

    let repository = r!(repository, repository)
        .filter(|s| !s.is_empty())
        .or_else(|| ops_git::GitInfo::collect(cwd).remote_url);

    let authors = pkg
        .and_then(|p| p.authors.value())
        .cloned()
        .or_else(|| {
            ws_pkg
                .filter(|wp| !wp.authors.is_empty())
                .map(|wp| wp.authors.clone())
        })
        .unwrap_or_default();

    ResolvedFields {
        version: r!(version, version),
        description: r!(description, description),
        edition: r!(edition, edition),
        license: r!(license, license),
        repository,
        homepage: r!(homepage, homepage),
        msrv: r!(rust_version, rust_version),
        authors,
    }
}

fn query_identity_metrics(ctx: &Context) -> IdentityMetrics {
    let (loc, file_count) = query_loc_from_db(ctx);
    let (coverage_percent, languages) = query_coverage_and_languages(ctx);
    IdentityMetrics {
        loc,
        file_count,
        dependency_count: query_dependency_count(ctx),
        coverage_percent,
        languages,
    }
}

impl DataProvider for RustIdentityProvider {
    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn about_fields(&self) -> Vec<AboutFieldDef> {
        let mut fields = base_about_fields();
        // Insert Rust-specific fields before "coverage" (index 6).
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
        fields.insert(
            insert_pos + 1,
            AboutFieldDef {
                id: "dependencies",
                label: "Dependencies",
                description: "Total dependency count",
            },
        );
        fields
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        // Parse Cargo.toml directly (don't depend on registry ordering).
        let cargo_toml_value = CargoTomlProvider::new().provide(ctx)?;
        let mut manifest: CargoToml = serde_json::from_value(cargo_toml_value)
            .map_err(DataProviderError::computation_error)?;

        let cwd = ctx.working_directory.clone();
        if let Some(ws) = &mut manifest.workspace {
            ws.members = resolve_member_globs(&ws.members, &cwd);
        }

        let pkg = manifest.package.as_ref();
        let ws_pkg = manifest.workspace.as_ref().and_then(|w| w.package.as_ref());

        let name = pkg
            .map(|p| p.name.clone())
            .unwrap_or_else(|| dir_name(&cwd).to_string());
        let fields = resolve_identity_fields(pkg, ws_pkg, &cwd);
        let metrics = query_identity_metrics(ctx);
        let module_count = manifest.workspace.as_ref().map(|w| w.members.len());
        let stack_detail = fields.edition.as_ref().map(|e| format!("Edition {e}"));

        let identity = ProjectIdentity {
            name,
            version: fields.version,
            description: fields.description,
            stack_label: "Rust".to_string(),
            stack_detail,
            license: fields.license,
            project_path: cwd.display().to_string(),
            module_count,
            module_label: "crates".to_string(),
            loc: metrics.loc,
            file_count: metrics.file_count,
            authors: fields.authors,
            repository: fields.repository,
            homepage: fields.homepage,
            msrv: fields.msrv,
            dependency_count: metrics.dependency_count,
            coverage_percent: metrics.coverage_percent,
            languages: metrics.languages,
        };

        serde_json::to_value(&identity).map_err(DataProviderError::from)
    }
}

/// Try to get dependency count from DuckDB.
fn query_dependency_count(ctx: &Context) -> Option<usize> {
    let db = ops_duckdb::get_db(ctx)?;
    ops_duckdb::sql::query_dependency_count(db).ok()
}

/// Try to get coverage percentage and language list from DuckDB.
fn query_coverage_and_languages(ctx: &Context) -> (Option<f64>, Vec<LanguageStat>) {
    let db = match ops_duckdb::get_db(ctx) {
        Some(db) => db,
        None => return (None, vec![]),
    };

    let coverage = ops_duckdb::sql::query_project_coverage(db)
        .ok()
        .filter(|c| c.lines_count > 0)
        .map(|c| c.lines_percent);

    let languages = ops_duckdb::sql::query_project_languages(db).unwrap_or_default();

    (coverage, languages)
}

/// Try to get project LOC and file count from DuckDB.
fn query_loc_from_db(ctx: &Context) -> (Option<i64>, Option<i64>) {
    let db = match ops_duckdb::get_db(ctx) {
        Some(db) => db,
        None => return (None, None),
    };

    let loc = ops_duckdb::sql::query_project_loc(db).ok();
    let files = ops_duckdb::sql::query_project_file_count(db).ok();
    (loc, files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_provider_name() {
        let provider = RustIdentityProvider;
        assert_eq!(provider.name(), "project_identity");
    }

    #[test]
    fn identity_about_fields_ids() {
        let provider = RustIdentityProvider;
        let fields = provider.about_fields();
        let ids: Vec<&str> = fields.iter().map(|f| f.id).collect();
        assert_eq!(
            ids,
            vec![
                "stack",
                "license",
                "project",
                "modules",
                "codebase",
                "repository",
                "authors",
                "homepage",
                "dependencies",
                "coverage",
            ]
        );
    }

    #[test]
    fn identity_provide_simple_package() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"
[package]
name = "my-crate"
version = "1.2.3"
edition = "2021"
license = "MIT"
description = "A test crate"
repository = "https://github.com/test/my-crate"
homepage = "https://my-crate.dev"
rust-version = "1.70"
authors = ["Alice <alice@example.com>"]
"#,
        )
        .unwrap();

        let provider = RustIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ops_core::project_identity::ProjectIdentity =
            serde_json::from_value(value).unwrap();

        assert_eq!(id.name, "my-crate");
        assert_eq!(id.version.as_deref(), Some("1.2.3"));
        assert_eq!(id.description.as_deref(), Some("A test crate"));
        assert_eq!(id.stack_label, "Rust");
        assert_eq!(id.stack_detail.as_deref(), Some("Edition 2021"));
        assert_eq!(id.license.as_deref(), Some("MIT"));
        assert_eq!(
            id.repository.as_deref(),
            Some("https://github.com/test/my-crate")
        );
        assert_eq!(id.homepage.as_deref(), Some("https://my-crate.dev"));
        assert_eq!(id.msrv.as_deref(), Some("1.70"));
        assert_eq!(id.authors, vec!["Alice <alice@example.com>"]);
        assert_eq!(id.module_label, "crates");
        assert!(id.module_count.is_none()); // no workspace
    }

    #[test]
    fn identity_manifest_repository_wins_over_git() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"
[package]
name = "my-crate"
version = "0.1.0"
repository = "https://example.com/manifest-wins"
"#,
        )
        .unwrap();
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        std::fs::write(
            git_dir.join("config"),
            "[remote \"origin\"]\n\turl = https://github.com/other/repo.git\n",
        )
        .unwrap();

        let provider = RustIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();

        assert_eq!(
            id.repository.as_deref(),
            Some("https://example.com/manifest-wins")
        );
    }

    #[test]
    fn identity_git_fills_repository_when_manifest_missing() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"my-crate\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        std::fs::write(
            git_dir.join("config"),
            "[remote \"origin\"]\n\turl = git@github.com:o/r.git\n",
        )
        .unwrap();

        let provider = RustIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();

        assert_eq!(id.repository.as_deref(), Some("https://github.com/o/r"));
    }

    #[test]
    fn identity_provide_workspace() {
        let dir = tempfile::tempdir().unwrap();
        // Create workspace members
        std::fs::create_dir_all(dir.path().join("crates/core")).unwrap();
        std::fs::write(
            dir.path().join("crates/core/Cargo.toml"),
            "[package]\nname = \"core\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("crates/cli")).unwrap();
        std::fs::write(
            dir.path().join("crates/cli/Cargo.toml"),
            "[package]\nname = \"cli\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"
[package]
name = "my-workspace"
version = "2.0.0"
edition = "2021"

[workspace]
members = ["crates/core", "crates/cli"]
"#,
        )
        .unwrap();

        let provider = RustIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ops_core::project_identity::ProjectIdentity =
            serde_json::from_value(value).unwrap();

        assert_eq!(id.name, "my-workspace");
        assert_eq!(id.module_count, Some(2));
    }

    #[test]
    fn identity_provide_workspace_with_globs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("crates/alpha")).unwrap();
        std::fs::write(
            dir.path().join("crates/alpha/Cargo.toml"),
            "[package]\nname = \"alpha\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("crates/beta")).unwrap();
        std::fs::write(
            dir.path().join("crates/beta/Cargo.toml"),
            "[package]\nname = \"beta\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("crates/not-a-crate")).unwrap();

        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"
[package]
name = "glob-ws"
version = "0.1.0"

[workspace]
members = ["crates/*"]
"#,
        )
        .unwrap();

        let provider = RustIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ops_core::project_identity::ProjectIdentity =
            serde_json::from_value(value).unwrap();

        assert_eq!(id.module_count, Some(2));
    }

    #[test]
    fn identity_provide_virtual_workspace() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("crates/lib")).unwrap();
        std::fs::write(
            dir.path().join("crates/lib/Cargo.toml"),
            "[package]\nname = \"lib\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"
[workspace]
members = ["crates/lib"]

[workspace.package]
version = "3.0.0"
edition = "2024"
description = "Virtual workspace desc"
license = "Apache-2.0"
repository = "https://github.com/test/vws"
authors = ["Bob"]
homepage = "https://vws.dev"
rust-version = "1.80"
"#,
        )
        .unwrap();

        let provider = RustIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ops_core::project_identity::ProjectIdentity =
            serde_json::from_value(value).unwrap();

        // Falls back to dir_name for name since no [package]
        assert_eq!(
            id.name,
            dir.path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string()
        );
        // Falls back to workspace.package for fields
        assert_eq!(id.version.as_deref(), Some("3.0.0"));
        assert_eq!(id.description.as_deref(), Some("Virtual workspace desc"));
        assert_eq!(id.stack_detail.as_deref(), Some("Edition 2024"));
        assert_eq!(id.license.as_deref(), Some("Apache-2.0"));
        assert_eq!(
            id.repository.as_deref(),
            Some("https://github.com/test/vws")
        );
        assert_eq!(id.homepage.as_deref(), Some("https://vws.dev"));
        assert_eq!(id.msrv.as_deref(), Some("1.80"));
        assert_eq!(id.authors, vec!["Bob"]);
    }

    #[test]
    fn identity_provide_minimal_package_has_no_enrichment() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"
[package]
name = "bare-crate"
version = "0.1.0"
edition = "2021"
"#,
        )
        .unwrap();

        let provider = RustIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ops_core::project_identity::ProjectIdentity =
            serde_json::from_value(value).unwrap();

        assert_eq!(id.name, "bare-crate");
        assert_eq!(id.version.as_deref(), Some("0.1.0"));
        assert_eq!(id.stack_label, "Rust");
        // Without DB, these are None
        assert!(id.loc.is_none());
        assert!(id.file_count.is_none());
        assert!(id.dependency_count.is_none());
        assert!(id.coverage_percent.is_none());
        assert!(id.languages.is_empty());
        assert!(id.module_count.is_none());
    }

    #[test]
    fn identity_provide_no_cargo_toml_fails() {
        let dir = tempfile::tempdir().unwrap();
        let provider = RustIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        assert!(provider.provide(&mut ctx).is_err());
    }

    fn parse_pkg(toml_str: &str) -> CargoToml {
        toml::from_str(toml_str).expect("test toml should parse")
    }

    #[test]
    fn resolve_field_prefers_package_over_workspace() {
        let manifest = parse_pkg(
            r#"
[package]
name = "test"
version = "1.0.0"
description = "pkg desc"

[workspace.package]
description = "ws desc"
"#,
        );
        let pkg = manifest.package.as_ref();
        let ws_pkg = manifest.workspace.as_ref().and_then(|w| w.package.as_ref());
        let result = resolve_field(
            pkg,
            ws_pkg,
            |p| p.description.as_str(),
            |wp| wp.description.as_deref(),
        );
        assert_eq!(result.as_deref(), Some("pkg desc"));
    }

    #[test]
    fn resolve_field_falls_back_to_workspace_when_no_package() {
        // When there is no [package] at all, resolve_field should use workspace
        let manifest = parse_pkg(
            r#"
[workspace.package]
description = "ws desc"
"#,
        );
        let ws_pkg = manifest.workspace.as_ref().and_then(|w| w.package.as_ref());
        let result = resolve_field(
            None,
            ws_pkg,
            |p: &ops_cargo_toml::Package| p.description.as_str(),
            |wp| wp.description.as_deref(),
        );
        assert_eq!(result.as_deref(), Some("ws desc"));
    }

    #[test]
    fn resolve_field_returns_none_when_both_none() {
        let result: Option<String> = resolve_field(
            None,
            None,
            |p: &ops_cargo_toml::Package| p.description.as_str(),
            |wp: &ops_cargo_toml::WorkspacePackage| wp.description.as_deref(),
        );
        assert!(result.is_none());
    }

    #[test]
    fn resolve_field_no_package_uses_workspace() {
        let manifest = parse_pkg(
            r#"
[workspace.package]
version = "2.0.0"
"#,
        );
        let ws_pkg = manifest.workspace.as_ref().and_then(|w| w.package.as_ref());
        let result = resolve_field(
            None,
            ws_pkg,
            |p: &ops_cargo_toml::Package| p.version.as_str(),
            |wp| wp.version.as_deref(),
        );
        assert_eq!(result.as_deref(), Some("2.0.0"));
    }

    #[test]
    fn resolve_field_no_package_no_workspace() {
        let result: Option<String> = resolve_field(
            None,
            None,
            |p: &ops_cargo_toml::Package| p.version.as_str(),
            |wp: &ops_cargo_toml::WorkspacePackage| wp.version.as_deref(),
        );
        assert!(result.is_none());
    }

    #[test]
    fn identity_provide_minimal_package_has_empty_optional_fields() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"
[package]
name = "minimal"
version = "0.1.0"
"#,
        )
        .unwrap();

        let provider = RustIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ops_core::project_identity::ProjectIdentity =
            serde_json::from_value(value).unwrap();

        assert_eq!(id.name, "minimal");
        assert_eq!(id.version.as_deref(), Some("0.1.0"));
        // With serde defaults, InheritableString fields default to empty string
        // which resolve_field treats as a valid (non-None) value
        assert!(id.authors.is_empty());
        assert!(id.module_count.is_none());
    }

    #[test]
    fn identity_provide_workspace_authors_fallback() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("crates/lib")).unwrap();
        std::fs::write(
            dir.path().join("crates/lib/Cargo.toml"),
            "[package]\nname = \"lib\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"
[workspace]
members = ["crates/lib"]

[workspace.package]
version = "1.0.0"
authors = ["Alice", "Bob"]
"#,
        )
        .unwrap();

        let provider = RustIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ops_core::project_identity::ProjectIdentity =
            serde_json::from_value(value).unwrap();

        assert_eq!(id.authors, vec!["Alice", "Bob"]);
    }

    #[test]
    fn identity_stack_label_is_always_rust() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let provider = RustIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ops_core::project_identity::ProjectIdentity =
            serde_json::from_value(value).unwrap();

        assert_eq!(id.stack_label, "Rust");
        assert_eq!(id.module_label, "crates");
    }
}
