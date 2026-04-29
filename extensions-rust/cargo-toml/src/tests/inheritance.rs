use super::*;

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
        crate::types::InheritableField::Value(vec!["Alice".to_string(), "Bob".to_string()])
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
        crate::types::InheritableField::Inherited { workspace: true }
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

/// ERR-1 regression (TASK-0555): with `default-features = false` set on the
/// workspace dep, a member that *re-enables* defaults locally still gets
/// `default-features = false` resolved — matching cargo's documented
/// behavior (member cannot un-disable workspace defaults).
#[test]
fn resolve_workspace_default_features_false_local_true_stays_false() {
    let toml = r#"
[package]
name = "member"
version = "0.1.0"

[dependencies]
tokio = { workspace = true, default-features = true }

[workspace.dependencies]
tokio = { version = "1", default-features = false, features = ["macros"] }
"#;
    let mut manifest = CargoToml::parse(toml).expect("should parse");
    manifest.resolve_inheritance().expect("should resolve");

    let tokio = &manifest.dependencies["tokio"];
    assert!(
        !tokio.uses_default_features(),
        "workspace default-features = false must win over local true"
    );
}

/// ERR-1 regression (TASK-0555): a non-optional workspace dep can be
/// opted-in to optional by an inheriting member.
#[test]
fn resolve_workspace_non_optional_local_optional_becomes_optional() {
    let toml = r#"
[package]
name = "member"
version = "0.1.0"

[dependencies]
serde = { workspace = true, optional = true }

[workspace.dependencies]
serde = { version = "1", optional = false }
"#;
    let mut manifest = CargoToml::parse(toml).expect("should parse");
    manifest.resolve_inheritance().expect("should resolve");

    let serde = &manifest.dependencies["serde"];
    assert!(
        serde.is_optional(),
        "local optional = true must take effect over a non-optional workspace dep"
    );
}

#[test]
fn inheritable_field_value_and_inherited() {
    let val: crate::types::InheritableField<String> =
        crate::types::InheritableField::Value("hello".to_string());
    assert_eq!(val.value(), Some(&"hello".to_string()));
    assert_eq!(val.as_str(), Some("hello"));

    let inherited: crate::types::InheritableField<String> =
        crate::types::InheritableField::Inherited { workspace: true };
    assert_eq!(inherited.value(), None);
    assert_eq!(inherited.as_str(), None);
}

#[test]
fn inheritable_field_default() {
    let field: crate::types::InheritableString = Default::default();
    assert_eq!(field.as_str(), Some(""));

    let vec_field: crate::types::InheritableVec = Default::default();
    assert_eq!(vec_field.value(), Some(&vec![]));
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
        crate::types::InheritableField::Inherited { workspace: true }
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
