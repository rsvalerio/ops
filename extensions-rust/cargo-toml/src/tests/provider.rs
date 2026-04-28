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
