use super::*;
use ops_core::config::Config;
use ops_extension::DataProviderError;
use std::sync::Arc;

fn test_context(working_dir: PathBuf) -> Context {
    Context::new(Arc::new(Config::empty()), working_dir)
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

    let err = provider.provide(&mut ctx).unwrap_err();
    assert!(
        matches!(err, DataProviderError::ComputationFailed(_)),
        "expected ComputationFailed for missing file, got: {err:?}"
    );
}

#[test]
fn provider_invalid_toml() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    std::fs::write(&cargo_toml, "not valid toml [[[").expect("write invalid toml");

    let provider = CargoTomlProvider::with_root(temp_dir.path().to_path_buf());
    let mut ctx = test_context(temp_dir.path().to_path_buf());

    let err = provider.provide(&mut ctx).unwrap_err();
    assert!(
        matches!(err, DataProviderError::ComputationFailed(_)),
        "expected ComputationFailed for invalid TOML, got: {err:?}"
    );
}

/// TQ-EFF-002: an unreadable manifest surfaces as a typed
/// [`DataProviderError::ComputationFailed`].
///
/// Unix-only: the mechanism is the DAC mode bits, and Windows manages file
/// access through ACLs.
///
/// TEST-18 / TASK-1802: the chmod is owned by a `Drop` guard so a panic in
/// the body still leaves the tempdir removable, and the guard probes whether
/// 0o000 is actually enforced — under `CAP_DAC_OVERRIDE` (uid 0, the default
/// in most CI container images) the read succeeds and the old
/// `assert!(result.is_err())` failed for a reason unrelated to crate logic.
#[cfg(unix)]
#[test]
fn provider_unreadable_file_returns_error() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    std::fs::write(
        &cargo_toml,
        "[package]\nname = \"test\"\nversion = \"0.1.0\"",
    )
    .expect("write cargo toml");

    let Some(_guard) = PermGuard::deny_all(&cargo_toml, |p| std::fs::read(p).map(|_| ())) else {
        skip_no_dac_enforcement("provider_unreadable_file_returns_error");
        return;
    };

    let provider = CargoTomlProvider::with_root(temp_dir.path().to_path_buf());
    let mut ctx = test_context(temp_dir.path().to_path_buf());

    let err = provider.provide(&mut ctx).unwrap_err();
    assert!(
        matches!(err, DataProviderError::ComputationFailed(_)),
        "expected ComputationFailed for an unreadable manifest, got: {err:?}"
    );
}

/// A manifest that populates every section the published schema documents.
const FULLY_POPULATED_MANIFEST: &str = r#"
[package]
name = "member"
version = "0.1.0"
edition = "2021"
authors = ["Alice"]
description = "A crate"
repository = "https://example.com/repo"
license = "MIT"

[dependencies]
serde = "1.0"

[dev-dependencies]
tempfile = "3"

[build-dependencies]
cc = "1"

[workspace]
members = ["."]

[workspace.dependencies]
serde = "1.0"
"#;

/// READ-6 / TASK-1798: hold the published schema and the emitted JSON
/// together. The previous assertion only checked that the schema *listed*
/// `dev-dependencies`, so it stayed green while serde serialised the field
/// as `dev_dependencies` and every consumer reading the documented key got
/// nothing.
#[test]
fn provider_schema_names_match_serialized_manifest() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    std::fs::write(temp_dir.path().join("Cargo.toml"), FULLY_POPULATED_MANIFEST)
        .expect("write cargo toml");

    let provider = CargoTomlProvider::with_root(temp_dir.path().to_path_buf());
    let mut ctx = test_context(temp_dir.path().to_path_buf());
    let value = provider.provide(&mut ctx).expect("should provide");

    let schema = provider.schema();
    assert!(!schema.fields.is_empty());

    for field in &schema.fields {
        // Dotted names denote a path into the emitted JSON; the leading
        // segment names the top-level key (`Package.version` →
        // `["package"]["version"]`).
        let mut cursor = &value;
        let mut walked = String::new();
        for (i, segment) in field.name.split('.').enumerate() {
            let key = if i == 0 {
                segment.to_ascii_lowercase()
            } else {
                segment.to_string()
            };
            walked.push('/');
            walked.push_str(&key);
            cursor = cursor.get(&key).unwrap_or_else(|| {
                panic!(
                    "schema field {:?} is not a key in the emitted JSON (missing at {walked}); \
                     schema and serde spelling have drifted",
                    field.name
                )
            });
        }
        assert!(
            !cursor.is_null(),
            "schema field {:?} serialised as null for a manifest that populates it",
            field.name
        );
    }
}

/// READ-6 / TASK-1798: the wire format accepts both spellings on the read
/// side, so a consumer that round-trips provider JSON back into `CargoToml`
/// keeps working, and so does a hand-written `snake_case` payload.
#[test]
fn provider_dependency_sections_round_trip_in_both_spellings() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    std::fs::write(temp_dir.path().join("Cargo.toml"), FULLY_POPULATED_MANIFEST)
        .expect("write cargo toml");

    let provider = CargoTomlProvider::with_root(temp_dir.path().to_path_buf());
    let mut ctx = test_context(temp_dir.path().to_path_buf());
    let value = provider.provide(&mut ctx).expect("should provide");

    assert!(
        value.get("dev-dependencies").is_some(),
        "serialised key must match the documented kebab-case spelling"
    );
    assert!(
        value.get("dev_dependencies").is_none(),
        "the Rust field name must not leak into the wire format"
    );

    let round_tripped: CargoToml =
        serde_json::from_value(value).expect("provider JSON must deserialize back");
    assert!(round_tripped.dev_dependencies.contains_key("tempfile"));
    assert!(round_tripped.build_dependencies.contains_key("cc"));

    let snake: CargoToml = serde_json::from_value(serde_json::json!({
        "dev_dependencies": { "tempfile": "3" },
        "build_dependencies": { "cc": "1" },
    }))
    .expect("snake_case payload must still deserialize");
    assert!(snake.dev_dependencies.contains_key("tempfile"));
    assert!(snake.build_dependencies.contains_key("cc"));
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
