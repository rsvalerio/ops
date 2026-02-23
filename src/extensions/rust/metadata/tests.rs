//! Tests for the metadata extension.

use super::*;
use crate::extension::{CommandRegistry, DataRegistry, Extension};

fn sample_metadata() -> serde_json::Value {
    serde_json::json!({
        "workspace_root": "/workspace",
        "target_directory": "/workspace/target",
        "build_directory": "/workspace/target/debug/build",
        "workspace_members": ["pkg-a 0.1.0 (path+file:///workspace/pkg-a)"],
        "workspace_default_members": ["pkg-a 0.1.0 (path+file:///workspace/pkg-a)"],
        "packages": [
            {
                "name": "pkg-a",
                "version": "0.1.0",
                "id": "pkg-a 0.1.0 (path+file:///workspace/pkg-a)",
                "edition": "2021",
                "manifest_path": "/workspace/pkg-a/Cargo.toml",
                "license": "MIT",
                "repository": "https://github.com/example/pkg-a",
                "description": "A sample package",
                "dependencies": [
                    {
                        "name": "serde",
                        "req": "^1.0",
                        "kind": null,
                        "optional": true,
                        "uses_default_features": true,
                        "features": ["derive"]
                    },
                    {
                        "name": "tokio",
                        "req": "^1.0",
                        "kind": "dev",
                        "optional": false,
                        "uses_default_features": false,
                        "features": []
                    },
                    {
                        "name": "cc",
                        "req": "^1.0",
                        "kind": "build",
                        "optional": false,
                        "uses_default_features": true,
                        "features": []
                    }
                ],
                "targets": [
                    {
                        "name": "pkg_a",
                        "kind": ["lib"],
                        "src_path": "/workspace/pkg-a/src/lib.rs"
                    },
                    {
                        "name": "pkg-a",
                        "kind": ["bin"],
                        "src_path": "/workspace/pkg-a/src/main.rs",
                        "required-features": ["default"]
                    }
                ]
            },
            {
                "name": "serde",
                "version": "1.0.0",
                "id": "serde 1.0.0 (registry+https://github.com/rust-lang/crates.io-index)",
                "edition": "2018",
                "manifest_path": "/cargo/registry/serde-1.0.0/Cargo.toml",
                "dependencies": [],
                "targets": []
            }
        ]
    })
}

fn test_pkg_a(metadata: &Metadata) -> Package<'_> {
    metadata.package_by_name("pkg-a").expect("fixture: pkg-a")
}

fn test_pkg_serde(metadata: &Metadata) -> Package<'_> {
    metadata.package_by_name("serde").expect("fixture: serde")
}

#[test]
fn metadata_extension_name() {
    assert_eq!(MetadataExtension.name(), "metadata");
}

#[test]
fn metadata_extension_registers_no_commands() {
    let mut registry = CommandRegistry::new();
    MetadataExtension.register_commands(&mut registry);
    assert!(registry.is_empty());
}

#[test]
fn metadata_extension_registers_data_provider() {
    let mut registry = DataRegistry::new();
    MetadataExtension.register_data_providers(&mut registry);
    assert!(registry.get("metadata").is_some());
}

#[test]
fn metadata_provider_name() {
    assert_eq!(MetadataProvider.name(), "metadata");
}

/// TQ-003: Integration test requiring external cargo binary.
///
/// This test is ignored because it requires:
/// 1. `cargo` to be available in PATH
/// 2. The test to run in a valid Cargo workspace
///
/// **Re-enable criteria:**
/// - Run with `cargo test -- --ignored` when cargo is available
/// - Or refactor to mock `cargo metadata` output using a trait
///
/// **Tracking:** This test validates real-world behavior and should be run
/// periodically in CI environments with cargo available.
#[test]
#[ignore = "TQ-003: requires external cargo binary; run with --ignored. Re-enable: when CI has cargo or refactor to mock"]
fn metadata_provider_returns_valid_json() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut ctx = Context::new(
        std::sync::Arc::new(crate::config::Config::default()),
        manifest_dir,
    );
    let value = MetadataProvider
        .provide(&mut ctx)
        .expect("cargo metadata should succeed");
    assert!(value.is_object());
    assert!(value.get("packages").is_some());
    assert!(value.get("workspace_root").is_some());
}

#[test]
fn metadata_provider_fails_in_non_cargo_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut ctx = Context::new(
        std::sync::Arc::new(crate::config::Config::default()),
        dir.path().to_path_buf(),
    );
    let result = MetadataProvider.provide(&mut ctx);
    assert!(result.is_err());
}

#[test]
fn metadata_workspace_root() {
    let m = Metadata::from_value(sample_metadata());
    assert_eq!(m.workspace_root(), "/workspace");
}

#[test]
fn metadata_target_directory() {
    let m = Metadata::from_value(sample_metadata());
    assert_eq!(m.target_directory(), "/workspace/target");
}

#[test]
fn metadata_build_directory() {
    let m = Metadata::from_value(sample_metadata());
    assert_eq!(m.build_directory(), Some("/workspace/target/debug/build"));
}

#[test]
fn metadata_packages_iterates_all() {
    let m = Metadata::from_value(sample_metadata());
    let names: Vec<&str> = m.packages().map(|p| p.name()).collect();
    assert_eq!(names, vec!["pkg-a", "serde"]);
}

#[test]
fn metadata_members_filters_workspace() {
    let m = Metadata::from_value(sample_metadata());
    let names: Vec<&str> = m.members().map(|p| p.name()).collect();
    assert_eq!(names, vec!["pkg-a"]);
}

#[test]
fn metadata_default_members_filters() {
    let m = Metadata::from_value(sample_metadata());
    let names: Vec<&str> = m.default_members().map(|p| p.name()).collect();
    assert_eq!(names, vec!["pkg-a"]);
}

#[test]
fn metadata_package_by_name_finds_match() {
    let m = Metadata::from_value(sample_metadata());
    let p = m.package_by_name("serde").expect("should find serde");
    assert_eq!(p.version(), "1.0.0");
}

#[test]
fn metadata_package_by_name_returns_none_for_missing() {
    let m = Metadata::from_value(sample_metadata());
    assert!(m.package_by_name("nonexistent").is_none());
}

#[test]
fn metadata_package_by_id() {
    let m = Metadata::from_value(sample_metadata());
    let p = m.package_by_id("serde 1.0.0 (registry+https://github.com/rust-lang/crates.io-index)");
    assert!(p.is_some());
}

#[test]
fn metadata_root_package_finds_match() {
    let m = Metadata::from_value(serde_json::json!({
        "workspace_root": "/workspace",
        "target_directory": "/workspace/target",
        "workspace_members": ["root-pkg 0.1.0 (path+file:///workspace)"],
        "packages": [
            {
                "name": "root-pkg",
                "version": "0.1.0",
                "id": "root-pkg 0.1.0 (path+file:///workspace)",
                "edition": "2021",
                "manifest_path": "/workspace/Cargo.toml",
                "dependencies": [],
                "targets": []
            }
        ]
    }));
    let root = m.root_package().expect("should find root package");
    assert_eq!(root.name(), "root-pkg");
    assert_eq!(root.manifest_path(), "/workspace/Cargo.toml");
}

#[test]
fn metadata_root_package_none_when_not_at_workspace_root() {
    let m = Metadata::from_value(sample_metadata());
    assert!(m.root_package().is_none());
}

#[test]
fn metadata_root_package_none_for_virtual_workspace() {
    let m = Metadata::from_value(serde_json::json!({
        "workspace_root": "/workspace",
        "target_directory": "/workspace/target",
        "workspace_members": [],
        "packages": []
    }));
    assert!(m.root_package().is_none());
}

#[test]
fn package_name_and_version() {
    let m = Metadata::from_value(sample_metadata());
    let p = test_pkg_a(&m);
    assert_eq!(p.name(), "pkg-a");
    assert_eq!(p.version(), "0.1.0");
}

#[test]
fn package_edition_and_manifest_path() {
    let m = Metadata::from_value(sample_metadata());
    let p = test_pkg_a(&m);
    assert_eq!(p.edition(), "2021");
    assert_eq!(p.manifest_path(), "/workspace/pkg-a/Cargo.toml");
}

#[test]
fn package_optional_fields() {
    let m = Metadata::from_value(sample_metadata());
    let p = test_pkg_a(&m);
    assert_eq!(p.license(), Some("MIT"));
    assert_eq!(p.repository(), Some("https://github.com/example/pkg-a"));
    assert_eq!(p.description(), Some("A sample package"));
}

#[test]
fn package_is_member() {
    let m = Metadata::from_value(sample_metadata());
    let pkg_a = test_pkg_a(&m);
    let serde = test_pkg_serde(&m);
    assert!(pkg_a.is_member());
    assert!(!serde.is_member());
}

#[test]
fn package_is_default_member() {
    let m = Metadata::from_value(sample_metadata());
    let pkg_a = test_pkg_a(&m);
    let serde = test_pkg_serde(&m);
    assert!(pkg_a.is_default_member());
    assert!(!serde.is_default_member());
}

#[test]
fn package_dependencies_filters_normal() {
    let m = Metadata::from_value(sample_metadata());
    let p = test_pkg_a(&m);
    let deps: Vec<&str> = p.dependencies().map(|d| d.name()).collect();
    assert_eq!(deps, vec!["serde"]);
}

#[test]
fn package_dev_dependencies() {
    let m = Metadata::from_value(sample_metadata());
    let p = test_pkg_a(&m);
    let deps: Vec<&str> = p.dev_dependencies().map(|d| d.name()).collect();
    assert_eq!(deps, vec!["tokio"]);
}

#[test]
fn package_build_dependencies() {
    let m = Metadata::from_value(sample_metadata());
    let p = test_pkg_a(&m);
    let deps: Vec<&str> = p.build_dependencies().map(|d| d.name()).collect();
    assert_eq!(deps, vec!["cc"]);
}

#[test]
fn package_all_dependencies() {
    let m = Metadata::from_value(sample_metadata());
    let p = test_pkg_a(&m);
    let deps: Vec<&str> = p.all_dependencies().map(|d| d.name()).collect();
    assert_eq!(deps, vec!["serde", "tokio", "cc"]);
}

#[test]
fn dependency_fields() {
    let m = Metadata::from_value(sample_metadata());
    let p = test_pkg_a(&m);
    let serde = p.dependencies().next().unwrap();
    assert_eq!(serde.name(), "serde");
    assert_eq!(serde.version_req(), "^1.0");
    assert_eq!(serde.kind(), DependencyKind::Normal);
    assert!(serde.is_optional());
    assert!(serde.uses_default_features());
    let features: Vec<&str> = serde.features().collect();
    assert_eq!(features, vec!["derive"]);
}

#[test]
fn dependency_kind_dev() {
    let m = Metadata::from_value(sample_metadata());
    let p = test_pkg_a(&m);
    let tokio = p.dev_dependencies().next().unwrap();
    assert_eq!(tokio.kind(), DependencyKind::Dev);
    assert!(!tokio.uses_default_features());
}

#[test]
fn dependency_kind_build() {
    let m = Metadata::from_value(sample_metadata());
    let p = test_pkg_a(&m);
    let cc = p.build_dependencies().next().unwrap();
    assert_eq!(cc.kind(), DependencyKind::Build);
}

#[test]
fn package_targets() {
    let m = Metadata::from_value(sample_metadata());
    let p = test_pkg_a(&m);
    let names: Vec<&str> = p.targets().map(|t| t.name()).collect();
    assert_eq!(names, vec!["pkg_a", "pkg-a"]);
}

#[test]
fn package_lib_target() {
    let m = Metadata::from_value(sample_metadata());
    let p = test_pkg_a(&m);
    let lib = p.lib_target().expect("should have lib");
    assert_eq!(lib.name(), "pkg_a");
    assert!(lib.is_lib());
    assert!(!lib.is_bin());
}

#[test]
fn package_bin_targets() {
    let m = Metadata::from_value(sample_metadata());
    let p = test_pkg_a(&m);
    let bins: Vec<&str> = p.bin_targets().map(|t| t.name()).collect();
    assert_eq!(bins, vec!["pkg-a"]);
}

#[test]
fn target_kinds() {
    let m = Metadata::from_value(sample_metadata());
    let p = test_pkg_a(&m);
    let lib = p.lib_target().unwrap();
    let kinds: Vec<&str> = lib.kinds().collect();
    assert_eq!(kinds, vec!["lib"]);
}

#[test]
fn target_src_path() {
    let m = Metadata::from_value(sample_metadata());
    let p = test_pkg_a(&m);
    let lib = p.lib_target().unwrap();
    assert_eq!(lib.src_path(), "/workspace/pkg-a/src/lib.rs");
}

#[test]
fn target_required_features() {
    let m = Metadata::from_value(sample_metadata());
    let p = test_pkg_a(&m);
    let bin = p.bin_targets().next().unwrap();
    let features: Vec<&str> = bin.required_features().collect();
    assert_eq!(features, vec!["default"]);
}

#[test]
fn target_type_checks() {
    let m = Metadata::from_value(sample_metadata());
    let p = test_pkg_a(&m);

    let lib = p.lib_target().unwrap();
    assert!(lib.is_lib());
    assert!(!lib.is_bin());
    assert!(!lib.is_test());
    assert!(!lib.is_example());
    assert!(!lib.is_bench());

    let bin = p.bin_targets().next().unwrap();
    assert!(!bin.is_lib());
    assert!(bin.is_bin());
    assert!(!bin.is_test());
}

#[test]
fn package_no_test_targets() {
    let m = Metadata::from_value(sample_metadata());
    let p = test_pkg_a(&m);
    assert_eq!(p.test_targets().count(), 0);
}

#[test]
fn package_no_example_targets() {
    let m = Metadata::from_value(sample_metadata());
    let p = test_pkg_a(&m);
    assert_eq!(p.example_targets().count(), 0);
}

#[test]
fn package_no_bench_targets() {
    let m = Metadata::from_value(sample_metadata());
    let p = test_pkg_a(&m);
    assert_eq!(p.bench_targets().count(), 0);
}

mod metadata_edge_case_tests {
    use super::*;

    #[test]
    fn metadata_build_directory_none_when_missing() {
        let m = Metadata::from_value(serde_json::json!({
            "workspace_root": "/workspace",
            "target_directory": "/workspace/target",
            "workspace_members": [],
            "packages": []
        }));
        assert!(m.build_directory().is_none());
    }

    #[test]
    fn metadata_empty_workspace_members() {
        let m = Metadata::from_value(serde_json::json!({
            "workspace_root": "/workspace",
            "target_directory": "/workspace/target",
            "workspace_members": [],
            "packages": []
        }));
        let members: Vec<_> = m.members().collect();
        assert!(members.is_empty(), "empty workspace should have no members");
    }

    #[test]
    fn metadata_package_no_targets() {
        let m = Metadata::from_value(serde_json::json!({
            "workspace_root": "/workspace",
            "target_directory": "/workspace/target",
            "workspace_members": ["empty-pkg 0.1.0 (path+file:///workspace/empty-pkg)"],
            "packages": [
                {
                    "name": "empty-pkg",
                    "version": "0.1.0",
                    "id": "empty-pkg 0.1.0 (path+file:///workspace/empty-pkg)",
                    "edition": "2021",
                    "manifest_path": "/workspace/empty-pkg/Cargo.toml",
                    "dependencies": [],
                    "targets": []
                }
            ]
        }));
        let pkg = m.package_by_name("empty-pkg").expect("should find package");
        assert_eq!(pkg.targets().count(), 0, "package should have no targets");
        assert!(pkg.lib_target().is_none());
        assert_eq!(pkg.bin_targets().count(), 0);
    }

    #[test]
    fn metadata_package_no_dependencies() {
        let m = Metadata::from_value(serde_json::json!({
            "workspace_root": "/workspace",
            "target_directory": "/workspace/target",
            "workspace_members": ["no-deps 0.1.0 (path+file:///workspace/no-deps)"],
            "packages": [
                {
                    "name": "no-deps",
                    "version": "0.1.0",
                    "id": "no-deps 0.1.0 (path+file:///workspace/no-deps)",
                    "edition": "2021",
                    "manifest_path": "/workspace/no-deps/Cargo.toml",
                    "dependencies": [],
                    "targets": []
                }
            ]
        }));
        let pkg = m.package_by_name("no-deps").expect("should find package");
        assert_eq!(pkg.all_dependencies().count(), 0);
        assert_eq!(pkg.dependencies().count(), 0);
        assert_eq!(pkg.dev_dependencies().count(), 0);
        assert_eq!(pkg.build_dependencies().count(), 0);
    }

    #[test]
    fn metadata_package_missing_optional_fields() {
        let m = Metadata::from_value(serde_json::json!({
            "workspace_root": "/workspace",
            "target_directory": "/workspace/target",
            "workspace_members": ["minimal 0.1.0 (path+file:///workspace/minimal)"],
            "packages": [
                {
                    "name": "minimal",
                    "version": "0.1.0",
                    "id": "minimal 0.1.0 (path+file:///workspace/minimal)",
                    "manifest_path": "/workspace/minimal/Cargo.toml",
                    "dependencies": [],
                    "targets": []
                }
            ]
        }));
        let pkg = m.package_by_name("minimal").expect("should find package");
        assert_eq!(
            pkg.edition(),
            "",
            "missing edition should fallback to empty"
        );
        assert!(pkg.license().is_none());
        assert!(pkg.repository().is_none());
        assert!(pkg.description().is_none());
    }

    #[test]
    fn dependency_missing_optional_fields() {
        let m = Metadata::from_value(serde_json::json!({
            "workspace_root": "/workspace",
            "target_directory": "/workspace/target",
            "workspace_members": ["pkg 0.1.0 (path+file:///workspace/pkg)"],
            "packages": [
                {
                    "name": "pkg",
                    "version": "0.1.0",
                    "id": "pkg 0.1.0 (path+file:///workspace/pkg)",
                    "manifest_path": "/workspace/pkg/Cargo.toml",
                    "dependencies": [
                        {
                            "name": "minimal-dep",
                            "req": "^1.0"
                        }
                    ],
                    "targets": []
                }
            ]
        }));
        let pkg = m.package_by_name("pkg").expect("should find package");
        let dep = pkg.dependencies().next().expect("should have dep");
        assert_eq!(dep.name(), "minimal-dep");
        assert!(
            !dep.is_optional(),
            "missing optional should default to false"
        );
        assert!(
            dep.uses_default_features(),
            "missing uses_default_features should default to true"
        );
        assert_eq!(dep.features().count(), 0);
        assert!(dep.rename().is_none());
        assert!(dep.target().is_none());
        assert!(dep.source().is_none());
    }
}
