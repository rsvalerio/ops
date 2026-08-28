use super::*;
use ops_core::config::Config;
use ops_extension::{DataRegistry, Extension};
use std::sync::Arc;

ops_extension::test_datasource_extension!(
    CargoTomlExtension::new(),
    name: "cargo-toml",
    data_provider: "cargo_toml"
);

/// TEST-11 / TASK-1800: drive the *registered* provider and assert it parsed
/// the manifest at the configured root.
///
/// The previous body asserted only `provider.name() == "cargo_toml"`, which
/// is the `DATA_PROVIDER_NAME` constant — the same value whichever root was
/// passed, and the same value `CargoTomlExtension::new()` registers. The
/// written manifest was dead setup, and replacing the `with_root` arm of
/// `register_data_providers` with the unconditional `CargoTomlProvider::new()`
/// branch left the test green. That wiring is how `about` and
/// `create-review-tasks` pin the provider to a root they resolved
/// themselves, and losing it silently sends both back to auto-discovery from
/// the working directory (the TASK-0501 "empty units/coverage" failure).
#[test]
fn extension_with_root_propagates_to_provider() {
    let root_dir = tempfile::tempdir().expect("create root temp dir");
    std::fs::write(
        root_dir.path().join("Cargo.toml"),
        r#"[package]
name = "test-crate"
version = "0.1.0"
"#,
    )
    .expect("write cargo toml");

    // A *different* tree, with its own manifest, used as the working
    // directory. Auto-discovery would resolve here; the configured root must
    // win instead.
    let cwd_dir = tempfile::tempdir().expect("create cwd temp dir");
    std::fs::write(
        cwd_dir.path().join("Cargo.toml"),
        r#"[package]
name = "wrong-crate"
version = "9.9.9"
"#,
    )
    .expect("write decoy cargo toml");

    let ext = CargoTomlExtension::with_root(root_dir.path().to_path_buf());
    let mut registry = DataRegistry::new();
    ext.register_data_providers(&mut registry);

    let provider = registry.get("cargo_toml").expect("provider registered");
    let mut ctx = Context::new(Arc::new(Config::empty()), cwd_dir.path().to_path_buf());
    let value = provider.provide(&mut ctx).expect("provider should provide");
    let manifest: CargoToml = serde_json::from_value(value).expect("should deserialize");

    assert_eq!(
        manifest.package_name(),
        Some("test-crate"),
        "the configured root must win over the context working directory"
    );
    assert_ne!(manifest.package_name(), Some("wrong-crate"));
}

/// TEST-11 / TASK-1800: the companion — with no configured root, the same
/// registration path *does* auto-discover from the working directory. Without
/// this, the assertion above could pass for a provider that ignored the
/// context entirely.
#[test]
fn extension_without_root_auto_discovers_from_working_directory() {
    let cwd_dir = tempfile::tempdir().expect("create cwd temp dir");
    std::fs::write(
        cwd_dir.path().join("Cargo.toml"),
        r#"[package]
name = "discovered-crate"
version = "0.1.0"
"#,
    )
    .expect("write cargo toml");

    let ext = CargoTomlExtension::new();
    let mut registry = DataRegistry::new();
    ext.register_data_providers(&mut registry);

    let provider = registry.get("cargo_toml").expect("provider registered");
    let mut ctx = Context::new(Arc::new(Config::empty()), cwd_dir.path().to_path_buf());
    let value = provider.provide(&mut ctx).expect("provider should provide");
    let manifest: CargoToml = serde_json::from_value(value).expect("should deserialize");

    assert_eq!(manifest.package_name(), Some("discovered-crate"));
}
