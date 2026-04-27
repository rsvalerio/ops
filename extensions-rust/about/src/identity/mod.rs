//! Rust-specific `project_identity` data provider.
//!
//! Reads Cargo.toml directly and queries DuckDB for LOC stats to build a
//! [`ProjectIdentity`](ops_core::project_identity::ProjectIdentity)
//! with Rust-specific fields (crates, edition, etc.).

mod metrics;
mod resolver;

use ops_about::identity::{build_identity_value, ParsedManifest};
use ops_core::project_identity::{base_about_fields, insert_homepage_field, AboutFieldDef};
use ops_extension::{Context, DataProvider, DataProviderError};

use crate::query::load_workspace_manifest;
use metrics::query_identity_metrics;
use resolver::resolve_identity_fields;

pub(crate) const PROVIDER_NAME: &str = "project_identity";

pub(crate) struct RustIdentityProvider;

impl DataProvider for RustIdentityProvider {
    fn name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn about_fields(&self) -> Vec<AboutFieldDef> {
        let mut fields = base_about_fields();
        insert_homepage_field(&mut fields);
        // Insert Rust-specific `dependencies` right after the homepage slot,
        // before `coverage` so the field ordering stays stable.
        let deps_pos = fields
            .iter()
            .position(|f| f.id == "coverage")
            .unwrap_or(fields.len());
        fields.insert(
            deps_pos,
            AboutFieldDef {
                id: "dependencies",
                label: "Dependencies",
                description: "Total dependency count",
            },
        );
        fields
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let manifest = load_workspace_manifest(ctx)?;
        let cwd = ctx.working_directory.clone();

        let pkg = manifest.package.as_ref();
        let ws_pkg = manifest.workspace.as_ref().and_then(|w| w.package.as_ref());
        let fields = resolve_identity_fields(pkg, ws_pkg, &cwd);
        let metrics = query_identity_metrics(ctx);

        build_identity_value(
            ParsedManifest {
                name: pkg.map(|p| p.name.clone()),
                version: fields.version,
                description: fields.description,
                license: fields.license,
                authors: fields.authors,
                homepage: fields.homepage,
                repository: fields.repository,
                stack_label: "Rust",
                stack_detail: fields.edition.as_ref().map(|e| format!("Edition {e}")),
                module_label: "crates",
                module_count: manifest.workspace.as_ref().map(|w| w.members.len()),
                loc: metrics.loc,
                file_count: metrics.file_count,
                msrv: fields.msrv,
                dependency_count: metrics.dependency_count,
                coverage_percent: metrics.coverage_percent,
                languages: metrics.languages,
            },
            &cwd,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_core::project_identity::ProjectIdentity;

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
        assert!(id.module_count.is_none());
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

    /// TASK-0375 AC#2: verify `[workspace].exclude` filters members through
    /// the identity provider (which feeds the same resolver as units/coverage).
    #[test]
    fn identity_provide_workspace_exclude() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["foo", "bar", "experimental"] {
            std::fs::create_dir_all(dir.path().join(format!("crates/{name}"))).unwrap();
            std::fs::write(
                dir.path().join(format!("crates/{name}/Cargo.toml")),
                format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n"),
            )
            .unwrap();
        }
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\nexclude = [\"crates/experimental\"]\n",
        )
        .unwrap();

        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = RustIdentityProvider.provide(&mut ctx).unwrap();
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

        assert_eq!(
            id.name,
            dir.path()
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string()
        );
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
