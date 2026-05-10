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

/// ERR-1 regression (TASK-0554): real-world Cargo manifests use the
/// kebab-case `default-features` key. Without a serde alias, the field
/// silently defaults back to `true`, masking the user's intent.
#[test]
fn dep_spec_detailed_default_features_false_kebab_case() {
    let toml = r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
serde = { version = "1.0", default-features = false }
"#;
    let manifest = CargoToml::parse(toml).expect("should parse");
    let serde = &manifest.dependencies["serde"];
    assert!(!serde.uses_default_features());
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
fn publish_spec_empty_registries() {
    let toml = r#"
[package]
name = "test"
version = "0.1.0"
publish = []
"#;
    let manifest = CargoToml::parse(toml).expect("should parse");
    assert_eq!(
        manifest.package.unwrap().publish.is_publishable(),
        Some(false),
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
    assert_eq!(
        manifest.package.unwrap().publish.is_publishable(),
        Some(true)
    );
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
