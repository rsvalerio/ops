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
        crate::types::InheritableField::Value("1.2.3".to_string())
    );
    assert_eq!(
        pkg.edition,
        crate::types::InheritableField::Value("2021".to_string())
    );
    assert_eq!(
        pkg.rust_version,
        crate::types::InheritableField::Value("1.70".to_string())
    );
    assert_eq!(
        pkg.authors,
        crate::types::InheritableField::Value(vec![
            "Alice <alice@example.com>".to_string(),
            "Bob".to_string()
        ])
    );
    assert_eq!(
        pkg.description,
        crate::types::InheritableField::Value("A fully specified crate".to_string())
    );
    assert_eq!(
        pkg.license,
        crate::types::InheritableField::Value("MIT OR Apache-2.0".to_string())
    );
    assert_eq!(
        pkg.keywords,
        crate::types::InheritableField::Value(vec!["example".to_string(), "test".to_string()])
    );
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
