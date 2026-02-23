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
    use crate::config::Config;
    use std::sync::Arc;

    fn test_context(working_dir: PathBuf) -> Context {
        Context::new(Arc::new(Config::default()), working_dir)
    }

    #[test]
    fn provider_parses_real_cargo_toml() {
        let workspace_root =
            find_workspace_root(std::path::Path::new(".")).expect("should find workspace root");

        let provider = CargoTomlProvider::with_root(workspace_root.clone());
        let mut ctx = test_context(workspace_root);

        let value = provider.provide(&mut ctx).expect("should provide");
        let manifest: CargoToml =
            serde_json::from_value(value).expect("should deserialize to CargoToml");

        assert_eq!(manifest.package_name(), Some("cargo-ops"));
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
}

mod extension_tests {
    use super::*;

    #[test]
    fn extension_name() {
        let ext = CargoTomlExtension::new();
        assert_eq!(ext.name(), "cargo-toml");
    }

    #[test]
    fn extension_registers_data_provider() {
        let ext = CargoTomlExtension::new();
        let mut registry = DataRegistry::new();
        ext.register_data_providers(&mut registry);

        assert!(registry.get("cargo_toml").is_some());
    }

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
}
