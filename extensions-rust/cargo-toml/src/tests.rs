//! Tests for cargo_toml extension.

use super::*;

mod types_tests {
    use super::*;

    #[test]
    fn parse_simple_package() {
        let toml = r#"
[package]
name = "my-crate"
version = "0.1.0"
edition = "2021"
"#;
        let manifest = CargoToml::parse(toml).expect("should parse");
        assert_eq!(manifest.package_name(), Some("my-crate"));
        assert_eq!(manifest.package_version(), Some("0.1.0"));
        assert!(!manifest.is_workspace());
        assert!(!manifest.is_virtual_workspace());
    }

    #[test]
    fn parse_package_with_all_fields() {
        let toml = r#"
[package]
name = "full-crate"
version = "1.2.3"
edition = "2021"
rust-version = "1.70"
authors = ["Alice <alice@example.com>", "Bob"]
description = "A fully specified crate"
documentation = "https://docs.rs/full-crate"
readme = "README.md"
homepage = "https://example.com"
repository = "https://github.com/example/full-crate"
license = "MIT OR Apache-2.0"
license-file = "LICENSE"
keywords = ["example", "test"]
categories = ["development-tools"]
default-run = "full-crate"
"#;
        let manifest = CargoToml::parse(toml).expect("should parse");
        let pkg = manifest.package.as_ref().expect("package exists");

        assert_eq!(pkg.name, "full-crate");
        assert_eq!(
            pkg.version,
            super::types::InheritableField::Value("1.2.3".to_string())
        );
        assert_eq!(
            pkg.edition,
            super::types::InheritableField::Value("2021".to_string())
        );
        assert_eq!(
            pkg.rust_version,
            super::types::InheritableField::Value("1.70".to_string())
        );
        assert_eq!(
            pkg.authors,
            super::types::InheritableField::Value(vec![
                "Alice <alice@example.com>".to_string(),
                "Bob".to_string()
            ])
        );
        assert_eq!(
            pkg.description,
            super::types::InheritableField::Value("A fully specified crate".to_string())
        );
        assert_eq!(
            pkg.license,
            super::types::InheritableField::Value("MIT OR Apache-2.0".to_string())
        );
        assert_eq!(pkg.keywords, vec!["example", "test"]);
    }

    #[test]
    fn parse_virtual_workspace() {
        let toml = r#"
[workspace]
members = ["crates/a", "crates/b"]
resolver = "2"
"#;
        let manifest = CargoToml::parse(toml).expect("should parse");
        assert!(manifest.is_virtual_workspace());
        assert!(manifest.is_workspace());
        assert!(manifest.package.is_none());

        let ws = manifest.workspace.as_ref().expect("workspace exists");
        assert_eq!(ws.members, vec!["crates/a", "crates/b"]);
        assert_eq!(ws.resolver, Some("2".to_string()));
    }

    #[test]
    fn parse_workspace_with_root_package() {
        let toml = r#"
[package]
name = "root"
version = "0.1.0"

[workspace]
members = ["crates/sub"]
"#;
        let manifest = CargoToml::parse(toml).expect("should parse");
        assert!(!manifest.is_virtual_workspace());
        assert!(manifest.is_workspace());
        assert!(manifest.package.is_some());
    }

    #[test]
    fn parse_simple_dependencies() {
        let toml = r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
serde = "1.0"
tokio = "1"
"#;
        let manifest = CargoToml::parse(toml).expect("should parse");

        assert_eq!(manifest.dependencies.len(), 2);
        assert!(manifest.dependencies.contains_key("serde"));
        assert!(manifest.dependencies.contains_key("tokio"));

        let serde = &manifest.dependencies["serde"];
        assert_eq!(serde.version(), Some("1.0"));
        assert!(serde.path().is_none());
        assert!(serde.git().is_none());
    }

    #[test]
    fn parse_detailed_dependencies() {
        let toml = r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
serde = { version = "1.0", features = ["derive"] }
my-local = { path = "../my-local" }
my-git = { git = "https://github.com/example/my-git", branch = "main" }
optional-dep = { version = "2.0", optional = true }
"#;
        let manifest = CargoToml::parse(toml).expect("should parse");

        let serde = &manifest.dependencies["serde"];
        assert_eq!(serde.version(), Some("1.0"));
        assert_eq!(serde.features(), &["derive"]);
        assert!(serde.uses_default_features());

        let my_local = &manifest.dependencies["my-local"];
        assert_eq!(my_local.path(), Some("../my-local"));

        let my_git = &manifest.dependencies["my-git"];
        assert_eq!(my_git.git(), Some("https://github.com/example/my-git"));

        let optional_dep = &manifest.dependencies["optional-dep"];
        assert!(optional_dep.is_optional());
    }

    #[test]
    fn parse_dev_and_build_dependencies() {
        let toml = r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
serde = "1.0"

[dev-dependencies]
tempfile = "3"

[build-dependencies]
cc = "1.0"
"#;
        let manifest = CargoToml::parse(toml).expect("should parse");

        assert_eq!(manifest.dependencies.len(), 1);
        assert!(manifest.dependencies.contains_key("serde"));

        assert_eq!(manifest.dev_dependencies.len(), 1);
        assert!(manifest.dev_dependencies.contains_key("tempfile"));

        assert_eq!(manifest.build_dependencies.len(), 1);
        assert!(manifest.build_dependencies.contains_key("cc"));
    }

    #[test]
    fn parse_features() {
        let toml = r#"
[package]
name = "test"
version = "0.1.0"

[features]
default = ["std", "derive"]
std = []
derive = ["serde/derive"]
full = ["std", "derive", "extra"]
"#;
        let manifest = CargoToml::parse(toml).expect("should parse");

        assert_eq!(manifest.features.len(), 4);
        assert_eq!(
            manifest.features.get("default"),
            Some(&vec!["std".to_string(), "derive".to_string()])
        );
        assert_eq!(
            manifest.features.get("derive"),
            Some(&vec!["serde/derive".to_string()])
        );
    }

    #[test]
    fn parse_workspace_dependencies() {
        let toml = r#"
[workspace]
members = ["crates/*"]

[workspace.dependencies]
serde = "1.0"
tokio = { version = "1", features = ["full"] }
"#;
        let manifest = CargoToml::parse(toml).expect("should parse");

        let ws = manifest.workspace.as_ref().expect("workspace");
        assert_eq!(ws.dependencies.len(), 2);
        assert!(ws.dependencies.contains_key("serde"));
    }

    #[test]
    fn resolve_workspace_inheritance_simple() {
        let toml = r#"
[package]
name = "member"
version = "0.1.0"

[dependencies]
serde = { workspace = true }

[workspace.dependencies]
serde = "1.0"
"#;
        let mut manifest = CargoToml::parse(toml).expect("should parse");
        manifest.resolve_inheritance().expect("should resolve");

        let serde = &manifest.dependencies["serde"];
        assert_eq!(serde.version(), Some("1.0"));
        assert!(!serde.is_workspace_inherited());
    }

    #[test]
    fn resolve_workspace_inheritance_with_local_features() {
        let toml = r#"
[package]
name = "member"
version = "0.1.0"

[dependencies]
serde = { workspace = true, features = ["derive"] }

[workspace.dependencies]
serde = { version = "1.0", features = ["std"] }
"#;
        let mut manifest = CargoToml::parse(toml).expect("should parse");
        manifest.resolve_inheritance().expect("should resolve");

        let serde = &manifest.dependencies["serde"];
        assert_eq!(serde.version(), Some("1.0"));

        let features = serde.features();
        assert!(features.contains(&"std".to_string()));
        assert!(features.contains(&"derive".to_string()));
    }

    #[test]
    fn resolve_inheritance_missing_workspace_dep() {
        let toml = r#"
[package]
name = "member"
version = "0.1.0"

[dependencies]
nonexistent = { workspace = true }

[workspace.dependencies]
serde = "1.0"
"#;
        let mut manifest = CargoToml::parse(toml).expect("should parse");
        let result = manifest.resolve_inheritance();
        assert!(result.is_err());

        match result {
            Err(InheritanceError::MissingWorkspaceDependency { name }) => {
                assert_eq!(name, "nonexistent");
            }
            _ => panic!("expected MissingWorkspaceDependency error"),
        }
    }

    #[test]
    fn publish_spec_variants() {
        let toml_default = r#"
[package]
name = "test"
version = "0.1.0"
"#;
        let manifest = CargoToml::parse(toml_default).expect("should parse");
        assert!(manifest.package.unwrap().publish.is_publishable());

        let toml_false = r#"
[package]
name = "test"
version = "0.1.0"
publish = false
"#;
        let manifest = CargoToml::parse(toml_false).expect("should parse");
        assert!(!manifest.package.unwrap().publish.is_publishable());

        let toml_registry = r#"
[package]
name = "test"
version = "0.1.0"
publish = ["my-registry"]
"#;
        let manifest = CargoToml::parse(toml_registry).expect("should parse");
        assert!(manifest.package.unwrap().publish.is_publishable());
    }

    #[test]
    fn readme_spec_variants() {
        let toml_path = r#"
[package]
name = "test"
version = "0.1.0"
readme = "README.md"
"#;
        let manifest = CargoToml::parse(toml_path).expect("should parse");
        match manifest.package.unwrap().readme {
            Some(ReadmeSpec::Path(p)) => assert_eq!(p, "README.md"),
            _ => panic!("expected Path variant"),
        }
    }

    #[test]
    fn dep_spec_package_rename() {
        let toml = r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
my-serde = { package = "serde", version = "1.0" }
"#;
        let manifest = CargoToml::parse(toml).expect("should parse");
        let dep = &manifest.dependencies["my-serde"];
        assert_eq!(dep.package(), Some("serde"));
    }
}

mod provider_tests {
    use super::*;
    use ops_core::config::Config;
    use std::sync::Arc;

    fn test_context(working_dir: PathBuf) -> Context {
        Context::new(Arc::new(Config::default()), working_dir)
    }

    #[test]
    fn provider_parses_real_cargo_toml() {
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

        let provider = CargoTomlProvider::with_root(workspace_root.clone());
        let mut ctx = test_context(workspace_root);

        let value = provider.provide(&mut ctx).expect("should provide");
        let manifest: CargoToml =
            serde_json::from_value(value).expect("should deserialize to CargoToml");

        assert_eq!(manifest.package_name(), Some("ops-cargo-toml"));
    }

    #[test]
    fn provider_missing_cargo_toml() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let provider = CargoTomlProvider::with_root(temp_dir.path().to_path_buf());
        let mut ctx = test_context(temp_dir.path().to_path_buf());

        let result = provider.provide(&mut ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("reading"));
    }

    #[test]
    fn provider_invalid_toml() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        std::fs::write(&cargo_toml, "not valid toml [[[").expect("write invalid toml");

        let provider = CargoTomlProvider::with_root(temp_dir.path().to_path_buf());
        let mut ctx = test_context(temp_dir.path().to_path_buf());

        let result = provider.provide(&mut ctx);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("parsing"));
    }

    /// TQ-EFF-002: Test that unreadable files return an error.
    ///
    /// This test is Unix-only because Windows has different permission semantics.
    /// On Unix, we can use `chmod 000` to make a file unreadable. On Windows,
    /// file permissions are managed via ACLs which would require a different approach.
    ///
    /// The test still runs on Windows but only verifies that the code doesn't panic
    /// (the permission modification and error assertion are Unix-specific).
    #[test]
    fn provider_unreadable_file_returns_error() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        std::fs::write(
            &cargo_toml,
            "[package]\nname = \"test\"\nversion = \"0.1.0\"",
        )
        .expect("write cargo toml");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&cargo_toml, std::fs::Permissions::from_mode(0o000)).ok();
        }

        let provider = CargoTomlProvider::with_root(temp_dir.path().to_path_buf());
        let mut ctx = test_context(temp_dir.path().to_path_buf());

        let result = provider.provide(&mut ctx);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&cargo_toml, std::fs::Permissions::from_mode(0o644)).ok();
        }

        #[cfg(unix)]
        assert!(result.is_err(), "unreadable file should return error");

        #[cfg(windows)]
        let _ = result;
    }

    #[test]
    fn provider_schema_has_expected_fields() {
        let provider = CargoTomlProvider::new();
        let schema = provider.schema();
        assert_eq!(
            schema.description,
            "Cargo.toml manifest data (parsed from workspace root)"
        );
        assert!(!schema.fields.is_empty());
        let field_names: Vec<&str> = schema.fields.iter().map(|f| f.name).collect();
        assert!(field_names.contains(&"package"));
        assert!(field_names.contains(&"workspace"));
        assert!(field_names.contains(&"dependencies"));
        assert!(field_names.contains(&"dev-dependencies"));
        assert!(field_names.contains(&"build-dependencies"));
        assert!(field_names.contains(&"Package.name"));
        assert!(field_names.contains(&"DepSpec"));
    }

    #[test]
    fn provider_resolve_root_auto_discovers() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        std::fs::write(
            &cargo_toml,
            "[package]\nname = \"test\"\nversion = \"0.1.0\"",
        )
        .expect("write cargo toml");

        let subdir = temp_dir.path().join("src");
        std::fs::create_dir_all(&subdir).expect("create subdir");

        // Provider with no explicit root should auto-discover
        let provider = CargoTomlProvider::new();
        let mut ctx = test_context(subdir);

        let value = provider.provide(&mut ctx).expect("should provide");
        let manifest: CargoToml = serde_json::from_value(value).expect("should deserialize");
        assert_eq!(manifest.package_name(), Some("test"));
    }

    #[test]
    fn provider_resolve_root_auto_discover_fails_without_cargo_toml() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let provider = CargoTomlProvider::new();
        let mut ctx = test_context(temp_dir.path().to_path_buf());

        let result = provider.provide(&mut ctx);
        assert!(result.is_err());
    }

    #[test]
    fn provider_resolves_inheritance_in_output() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        std::fs::write(
            &cargo_toml,
            r#"
[package]
name = "member"
version = { workspace = true }
edition = { workspace = true }

[dependencies]
serde = { workspace = true }

[workspace]
members = []

[workspace.package]
version = "2.0.0"
edition = "2024"

[workspace.dependencies]
serde = "1.0"
"#,
        )
        .expect("write cargo toml");

        let provider = CargoTomlProvider::with_root(temp_dir.path().to_path_buf());
        let mut ctx = test_context(temp_dir.path().to_path_buf());

        let value = provider.provide(&mut ctx).expect("should provide");
        let manifest: CargoToml = serde_json::from_value(value).expect("should deserialize");

        // Both dep inheritance and package inheritance should be resolved
        assert_eq!(manifest.dependencies["serde"].version(), Some("1.0"));
        assert_eq!(
            manifest.package.as_ref().unwrap().version.as_str(),
            Some("2.0.0")
        );
        assert_eq!(
            manifest.package.as_ref().unwrap().edition.as_str(),
            Some("2024")
        );
    }
}

mod extension_tests {
    use super::*;
    use ops_extension::{DataRegistry, Extension};

    ops_extension::test_datasource_extension!(
        CargoTomlExtension::new(),
        name: "cargo-toml",
        data_provider: "cargo_toml"
    );

    #[test]
    fn extension_with_root_propagates_to_provider() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        std::fs::write(
            &cargo_toml,
            r#"[package]
name = "test-crate"
version = "0.1.0"
"#,
        )
        .expect("write cargo toml");

        let ext = CargoTomlExtension::with_root(temp_dir.path().to_path_buf());
        let mut registry = DataRegistry::new();
        ext.register_data_providers(&mut registry);

        let provider = registry.get("cargo_toml").expect("provider registered");
        assert_eq!(provider.name(), "cargo_toml");
    }
}

mod find_root_tests {
    use super::*;
    use std::fs;

    #[test]
    fn find_root_in_current_dir() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(&cargo_toml, "[package]\nname = \"test\"\n").expect("write cargo toml");

        let root = find_workspace_root(temp_dir.path()).expect("should find");
        assert_eq!(root, temp_dir.path());
    }

    #[test]
    fn find_root_in_parent() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let cargo_toml = temp_dir.path().join("Cargo.toml");
        fs::write(&cargo_toml, "[package]\nname = \"test\"\n").expect("write cargo toml");

        let subdir = temp_dir.path().join("crates").join("sub");
        fs::create_dir_all(&subdir).expect("create subdir");

        let root = find_workspace_root(&subdir).expect("should find");
        assert_eq!(root, temp_dir.path());
    }

    #[test]
    fn find_root_not_found() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");

        let result = find_workspace_root(temp_dir.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("no Cargo.toml found"));
    }
}

mod cargo_toml_edge_case_tests {
    use super::*;

    #[test]
    fn parse_with_lib_and_multiple_bins() {
        let toml = r#"
[package]
name = "multi-target"
version = "0.1.0"

[lib]
name = "multi_target"
path = "src/lib.rs"

[[bin]]
name = "cli1"
path = "src/bin/cli1.rs"

[[bin]]
name = "cli2"
path = "src/bin/cli2.rs"

[[test]]
name = "integration"
path = "tests/integration.rs"

[[example]]
name = "demo"
path = "examples/demo.rs"

[[bench]]
name = "perf"
path = "benches/perf.rs"
"#;
        let manifest = CargoToml::parse(toml).expect("should parse");
        assert_eq!(manifest.package_name(), Some("multi-target"));
    }

    #[test]
    fn parse_with_missing_required_name() {
        let toml = r#"
[package]
version = "0.1.0"
"#;
        let result = CargoToml::parse(toml);
        assert!(result.is_err(), "missing name should fail to parse");
    }

    #[test]
    fn parse_with_missing_required_version() {
        let toml = r#"
[package]
name = "test"
"#;
        let manifest = CargoToml::parse(toml).expect("should parse with default version");
        assert_eq!(manifest.package_name(), Some("test"));
        assert_eq!(manifest.package_version(), Some(""));
    }

    #[test]
    fn parse_with_empty_package_name() {
        let toml = r#"
[package]
name = ""
version = "0.1.0"
"#;
        let manifest = CargoToml::parse(toml);
        assert!(
            manifest.is_ok(),
            "empty name should parse (validation is Cargo's job)"
        );
        let m = manifest.unwrap();
        assert_eq!(m.package_name(), Some(""));
    }

    #[test]
    fn parse_with_target_specific_dependencies() {
        let toml = r#"
[package]
name = "test"
version = "0.1.0"

[target.'cfg(windows)'.dependencies]
winapi = "0.3"

[target.'cfg(unix)'.dependencies]
libc = "0.2"
"#;
        let manifest = CargoToml::parse(toml).expect("should parse");
        assert_eq!(manifest.package_name(), Some("test"));
    }

    #[test]
    fn parse_with_profile_settings() {
        let toml = r#"
[package]
name = "test"
version = "0.1.0"

[profile.release]
lto = true
opt-level = 3

[profile.dev]
opt-level = 0
"#;
        let manifest = CargoToml::parse(toml).expect("should parse");
        assert_eq!(manifest.package_name(), Some("test"));
    }

    #[test]
    fn parse_minimal_valid() {
        let toml = r#"
[package]
name = "a"
version = "0.1.0"
"#;
        let manifest = CargoToml::parse(toml).expect("should parse minimal");
        assert_eq!(manifest.package_name(), Some("a"));
        assert_eq!(manifest.package_version(), Some("0.1.0"));
        assert!(manifest.dependencies.is_empty());
        assert!(manifest.features.is_empty());
    }

    #[test]
    fn resolve_deeply_nested_workspace_inheritance() {
        let toml = r#"
[package]
name = "member"
version = "0.1.0"

[dependencies]
serde = { workspace = true }
tokio = { workspace = true }

[workspace.dependencies]
serde = { version = "1.0", features = ["std"] }
tokio = { version = "1", features = ["net", "time", "sync", "rt", "macros"] }
"#;
        let mut manifest = CargoToml::parse(toml).expect("should parse");
        manifest.resolve_inheritance().expect("should resolve");

        let serde = &manifest.dependencies["serde"];
        assert_eq!(serde.version(), Some("1.0"));
        assert!(serde.features().contains(&"std".to_string()));

        let tokio = &manifest.dependencies["tokio"];
        assert_eq!(tokio.version(), Some("1"));
        let tokio_features = tokio.features();
        assert!(tokio_features.contains(&"net".to_string()));
        assert!(tokio_features.contains(&"time".to_string()));
    }

    #[test]
    fn resolve_inheritance_with_many_deps() {
        let toml = r#"
[package]
name = "member"
version = "0.1.0"

[dependencies]
a = { workspace = true }
b = { workspace = true }
c = { workspace = true }

[workspace.dependencies]
a = "1.0"
b = "2.0"
c = "3.0"
"#;
        let mut manifest = CargoToml::parse(toml).expect("should parse");
        manifest.resolve_inheritance().expect("should resolve");

        assert_eq!(manifest.dependencies.len(), 3);
        assert_eq!(manifest.dependencies["a"].version(), Some("1.0"));
        assert_eq!(manifest.dependencies["b"].version(), Some("2.0"));
        assert_eq!(manifest.dependencies["c"].version(), Some("3.0"));
    }

    #[test]
    fn resolve_package_inheritance_version_and_edition() {
        let toml = r#"
[package]
name = "member"
version = { workspace = true }
edition = { workspace = true }
description = { workspace = true }
license = { workspace = true }
repository = { workspace = true }

[workspace]
members = ["crates/*"]

[workspace.package]
version = "2.0.0"
edition = "2024"
description = "Shared description"
license = "MIT"
repository = "https://github.com/example/repo"
"#;
        let mut manifest = CargoToml::parse(toml).expect("should parse");
        manifest.resolve_package_inheritance();

        let pkg = manifest.package.as_ref().unwrap();
        assert_eq!(pkg.version.as_str(), Some("2.0.0"));
        assert_eq!(pkg.edition.as_str(), Some("2024"));
        assert_eq!(pkg.description.as_str(), Some("Shared description"));
        assert_eq!(pkg.license.as_str(), Some("MIT"));
        assert_eq!(
            pkg.repository.as_str(),
            Some("https://github.com/example/repo")
        );
    }

    #[test]
    fn resolve_package_inheritance_authors() {
        let toml = r#"
[package]
name = "member"
version = "0.1.0"
authors = { workspace = true }

[workspace]
members = []

[workspace.package]
authors = ["Alice", "Bob"]
"#;
        let mut manifest = CargoToml::parse(toml).expect("should parse");
        manifest.resolve_package_inheritance();

        let pkg = manifest.package.as_ref().unwrap();
        assert_eq!(
            pkg.authors,
            super::types::InheritableField::Value(vec!["Alice".to_string(), "Bob".to_string()])
        );
    }

    #[test]
    fn resolve_package_inheritance_no_workspace_package() {
        let toml = r#"
[package]
name = "member"
version = { workspace = true }

[workspace]
members = []
"#;
        let mut manifest = CargoToml::parse(toml).expect("should parse");
        manifest.resolve_package_inheritance();

        // Version should remain inherited (unresolved) since no workspace.package exists
        let pkg = manifest.package.as_ref().unwrap();
        assert_eq!(
            pkg.version,
            super::types::InheritableField::Inherited { workspace: true }
        );
    }

    #[test]
    fn resolve_package_inheritance_no_package() {
        let toml = r#"
[workspace]
members = ["crates/*"]

[workspace.package]
version = "1.0.0"
"#;
        let mut manifest = CargoToml::parse(toml).expect("should parse");
        // Should not panic when there's no package section
        manifest.resolve_package_inheritance();
        assert!(manifest.package.is_none());
    }

    #[test]
    fn resolve_inheritance_no_workspace() {
        let toml = r#"
[package]
name = "standalone"
version = "0.1.0"

[dependencies]
serde = "1.0"
"#;
        let mut manifest = CargoToml::parse(toml).expect("should parse");
        // Should succeed with no workspace section
        manifest.resolve_inheritance().expect("should resolve");
        assert_eq!(manifest.dependencies["serde"].version(), Some("1.0"));
    }

    #[test]
    fn resolve_inheritance_dev_and_build_deps() {
        let toml = r#"
[package]
name = "member"
version = "0.1.0"

[dev-dependencies]
tempfile = { workspace = true }

[build-dependencies]
cc = { workspace = true }

[workspace.dependencies]
tempfile = "3.0"
cc = { version = "1.0", features = ["parallel"] }
"#;
        let mut manifest = CargoToml::parse(toml).expect("should parse");
        manifest.resolve_inheritance().expect("should resolve");

        assert_eq!(manifest.dev_dependencies["tempfile"].version(), Some("3.0"));
        assert_eq!(manifest.build_dependencies["cc"].version(), Some("1.0"));
        assert!(manifest.build_dependencies["cc"]
            .features()
            .contains(&"parallel".to_string()));
    }

    #[test]
    fn resolve_workspace_inheritance_with_optional_override() {
        let toml = r#"
[package]
name = "member"
version = "0.1.0"

[dependencies]
serde = { workspace = true, optional = true }

[workspace.dependencies]
serde = { version = "1.0", features = ["std"] }
"#;
        let mut manifest = CargoToml::parse(toml).expect("should parse");
        manifest.resolve_inheritance().expect("should resolve");

        let serde = &manifest.dependencies["serde"];
        assert!(serde.is_optional());
        assert_eq!(serde.version(), Some("1.0"));
    }

    #[test]
    fn resolve_workspace_inheritance_default_features_false() {
        let toml = r#"
[package]
name = "member"
version = "0.1.0"

[dependencies]
tokio = { workspace = true, default_features = false }

[workspace.dependencies]
tokio = { version = "1", default_features = true, features = ["full"] }
"#;
        let mut manifest = CargoToml::parse(toml).expect("should parse");
        manifest.resolve_inheritance().expect("should resolve");

        let tokio = &manifest.dependencies["tokio"];
        assert!(!tokio.uses_default_features());
    }

    #[test]
    fn inheritable_field_value_and_inherited() {
        let val: super::types::InheritableField<String> =
            super::types::InheritableField::Value("hello".to_string());
        assert_eq!(val.value(), Some(&"hello".to_string()));
        assert_eq!(val.as_str(), Some("hello"));

        let inherited: super::types::InheritableField<String> =
            super::types::InheritableField::Inherited { workspace: true };
        assert_eq!(inherited.value(), None);
        assert_eq!(inherited.as_str(), None);
    }

    #[test]
    fn inheritable_field_default() {
        let field: super::types::InheritableString = Default::default();
        assert_eq!(field.as_str(), Some(""));

        let vec_field: super::types::InheritableVec = Default::default();
        assert_eq!(vec_field.value(), Some(&vec![]));
    }

    #[test]
    fn dep_spec_simple_accessors() {
        let simple = DepSpec::Simple("1.0".to_string());
        assert_eq!(simple.version(), Some("1.0"));
        assert!(simple.detail().is_none());
        assert!(simple.path().is_none());
        assert!(simple.git().is_none());
        assert!(simple.features().is_empty());
        assert!(!simple.is_optional());
        assert!(simple.uses_default_features());
        assert!(simple.package().is_none());
        assert!(!simple.is_workspace_inherited());
    }

    #[test]
    fn dep_spec_detailed_default_features_false() {
        let toml = r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
serde = { version = "1.0", default_features = false }
"#;
        let manifest = CargoToml::parse(toml).expect("should parse");
        let serde = &manifest.dependencies["serde"];
        assert!(!serde.uses_default_features());
    }

    #[test]
    fn readme_spec_bool_variant() {
        let toml = r#"
[package]
name = "test"
version = "0.1.0"
readme = false
"#;
        let manifest = CargoToml::parse(toml).expect("should parse");
        match manifest.package.unwrap().readme {
            Some(ReadmeSpec::Bool(b)) => assert!(!b),
            _ => panic!("expected Bool variant"),
        }
    }

    #[test]
    fn readme_spec_true_variant() {
        let toml = r#"
[package]
name = "test"
version = "0.1.0"
readme = true
"#;
        let manifest = CargoToml::parse(toml).expect("should parse");
        match manifest.package.unwrap().readme {
            Some(ReadmeSpec::Bool(b)) => assert!(b),
            _ => panic!("expected Bool(true) variant"),
        }
    }

    #[test]
    fn publish_spec_empty_registries() {
        let toml = r#"
[package]
name = "test"
version = "0.1.0"
publish = []
"#;
        let manifest = CargoToml::parse(toml).expect("should parse");
        assert!(
            !manifest.package.unwrap().publish.is_publishable(),
            "empty registries should not be publishable"
        );
    }

    #[test]
    fn publish_spec_true() {
        let toml = r#"
[package]
name = "test"
version = "0.1.0"
publish = true
"#;
        let manifest = CargoToml::parse(toml).expect("should parse");
        assert!(manifest.package.unwrap().publish.is_publishable());
    }

    #[test]
    fn workspace_exclude_and_default_members() {
        let toml = r#"
[workspace]
members = ["crates/*"]
default-members = ["crates/core"]
exclude = ["crates/experimental"]
resolver = "2"
"#;
        let manifest = CargoToml::parse(toml).expect("should parse");
        let ws = manifest.workspace.as_ref().unwrap();
        assert_eq!(ws.default_members, vec!["crates/core"]);
        assert_eq!(ws.exclude, vec!["crates/experimental"]);
        assert_eq!(ws.resolver, Some("2".to_string()));
    }

    #[test]
    fn dep_spec_git_with_branch_tag_rev() {
        let toml = r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
my-git = { git = "https://github.com/example/repo", branch = "dev", tag = "v1.0", rev = "abc123" }
"#;
        let manifest = CargoToml::parse(toml).expect("should parse");
        let dep = &manifest.dependencies["my-git"];
        let detail = dep.detail().expect("should be detailed");
        assert_eq!(
            detail.git.as_deref(),
            Some("https://github.com/example/repo")
        );
        assert_eq!(detail.branch.as_deref(), Some("dev"));
        assert_eq!(detail.tag.as_deref(), Some("v1.0"));
        assert_eq!(detail.rev.as_deref(), Some("abc123"));
    }

    #[test]
    fn workspace_members_accessor() {
        let toml = r#"
[workspace]
members = ["crates/a", "crates/b", "extensions/*"]
"#;
        let manifest = CargoToml::parse(toml).expect("should parse");
        let members = manifest.workspace_members().expect("has workspace");
        assert_eq!(members, &["crates/a", "crates/b", "extensions/*"]);
    }

    #[test]
    fn workspace_members_none_without_workspace() {
        let toml = r#"
[package]
name = "standalone"
version = "0.1.0"
"#;
        let manifest = CargoToml::parse(toml).expect("should parse");
        assert!(manifest.workspace_members().is_none());
    }

    #[test]
    fn readme_spec_table_variant() {
        let toml = r#"
[package]
name = "test"
version = "0.1.0"
readme = { file = "docs/README.md" }
"#;
        let manifest = CargoToml::parse(toml).expect("should parse");
        match manifest.package.unwrap().readme {
            Some(ReadmeSpec::Table { file }) => assert_eq!(file, "docs/README.md"),
            other => panic!("expected Table variant, got {:?}", other),
        }
    }

    #[test]
    fn merge_features_deduplicates() {
        let toml = r#"
[package]
name = "member"
version = "0.1.0"

[dependencies]
serde = { workspace = true, features = ["std", "extra"] }

[workspace.dependencies]
serde = { version = "1.0", features = ["std", "derive"] }
"#;
        let mut manifest = CargoToml::parse(toml).expect("should parse");
        manifest.resolve_inheritance().expect("should resolve");

        let features = manifest.dependencies["serde"].features();
        // "std" appears in both but should only appear once
        assert_eq!(features.iter().filter(|f| *f == "std").count(), 1);
        assert!(features.contains(&"derive".to_string()));
        assert!(features.contains(&"extra".to_string()));
    }

    #[test]
    fn resolve_simple_ws_dep_with_local_optional_and_features() {
        let toml = r#"
[package]
name = "member"
version = "0.1.0"

[dependencies]
serde = { workspace = true, optional = true, features = ["derive"] }

[workspace.dependencies]
serde = "1.0"
"#;
        let mut manifest = CargoToml::parse(toml).expect("should parse");
        manifest.resolve_inheritance().expect("should resolve");

        let serde = &manifest.dependencies["serde"];
        assert_eq!(serde.version(), Some("1.0"));
        assert!(serde.is_optional());
        assert_eq!(serde.features(), &["derive"]);
    }

    #[test]
    fn resolve_detailed_ws_dep_propagates_git_fields() {
        let toml = r#"
[package]
name = "member"
version = "0.1.0"

[dependencies]
my-crate = { workspace = true }

[workspace.dependencies]
my-crate = { git = "https://github.com/example/repo", branch = "main", tag = "v1", rev = "abc", package = "actual-name", target = "cfg(unix)" }
"#;
        let mut manifest = CargoToml::parse(toml).expect("should parse");
        manifest.resolve_inheritance().expect("should resolve");

        let dep = &manifest.dependencies["my-crate"];
        let detail = dep.detail().expect("should be detailed");
        assert_eq!(
            detail.git.as_deref(),
            Some("https://github.com/example/repo")
        );
        assert_eq!(detail.branch.as_deref(), Some("main"));
        assert_eq!(detail.tag.as_deref(), Some("v1"));
        assert_eq!(detail.rev.as_deref(), Some("abc"));
        assert_eq!(detail.package.as_deref(), Some("actual-name"));
        assert_eq!(detail.target.as_deref(), Some("cfg(unix)"));
    }

    #[test]
    fn resolve_detailed_ws_dep_optional_or_logic() {
        let toml = r#"
[package]
name = "member"
version = "0.1.0"

[dependencies]
dep-a = { workspace = true }
dep-b = { workspace = true, optional = true }

[workspace.dependencies]
dep-a = { version = "1.0", optional = true }
dep-b = { version = "2.0" }
"#;
        let mut manifest = CargoToml::parse(toml).expect("should parse");
        manifest.resolve_inheritance().expect("should resolve");

        // optional in ws → optional in resolved
        assert!(manifest.dependencies["dep-a"].is_optional());
        // optional in local → optional in resolved
        assert!(manifest.dependencies["dep-b"].is_optional());
    }

    #[test]
    fn resolve_package_inheritance_all_string_fields() {
        let toml = r#"
[package]
name = "member"
version = { workspace = true }
edition = { workspace = true }
rust-version = { workspace = true }
description = { workspace = true }
documentation = { workspace = true }
homepage = { workspace = true }
repository = { workspace = true }
license = { workspace = true }

[workspace]
members = []

[workspace.package]
version = "3.0.0"
edition = "2024"
rust-version = "1.80"
description = "Shared desc"
documentation = "https://docs.example.com"
homepage = "https://example.com"
repository = "https://github.com/example/repo"
license = "Apache-2.0"
"#;
        let mut manifest = CargoToml::parse(toml).expect("should parse");
        manifest.resolve_package_inheritance();

        let pkg = manifest.package.as_ref().unwrap();
        assert_eq!(pkg.version.as_str(), Some("3.0.0"));
        assert_eq!(pkg.edition.as_str(), Some("2024"));
        assert_eq!(pkg.rust_version.as_str(), Some("1.80"));
        assert_eq!(pkg.description.as_str(), Some("Shared desc"));
        assert_eq!(pkg.documentation.as_str(), Some("https://docs.example.com"));
        assert_eq!(pkg.homepage.as_str(), Some("https://example.com"));
        assert_eq!(
            pkg.repository.as_str(),
            Some("https://github.com/example/repo")
        );
        assert_eq!(pkg.license.as_str(), Some("Apache-2.0"));
    }

    #[test]
    fn resolve_package_inheritance_missing_ws_value_stays_inherited() {
        let toml = r#"
[package]
name = "member"
version = { workspace = true }
edition = "2021"

[workspace]
members = []

[workspace.package]
edition = "2024"
"#;
        let mut manifest = CargoToml::parse(toml).expect("should parse");
        manifest.resolve_package_inheritance();

        let pkg = manifest.package.as_ref().unwrap();
        // version has workspace=true but ws.package has no version → stays inherited
        assert_eq!(
            pkg.version,
            super::types::InheritableField::Inherited { workspace: true }
        );
        // edition was already a direct value, should remain unchanged
        assert_eq!(pkg.edition.as_str(), Some("2021"));
    }

    #[test]
    fn resolve_package_inheritance_no_workspace() {
        let toml = r#"
[package]
name = "standalone"
version = "1.0.0"
"#;
        let mut manifest = CargoToml::parse(toml).expect("should parse");
        // Should not panic with no workspace section
        manifest.resolve_package_inheritance();
        assert_eq!(manifest.package_version(), Some("1.0.0"));
    }
}
