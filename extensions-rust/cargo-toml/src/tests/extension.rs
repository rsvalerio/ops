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
