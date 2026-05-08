//! Tests for the metadata extension.

use super::*;
ops_extension::test_datasource_extension!(
    MetadataExtension,
    name: "metadata",
    data_provider: "metadata"
);

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
fn metadata_provider_returns_valid_json() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut ctx = Context::test_context(manifest_dir);
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
    let mut ctx = Context::test_context(dir.path().to_path_buf());
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

#[cfg(windows)]
#[test]
fn metadata_root_package_finds_match_with_backslash_separator() {
    // TASK-0952: on Windows, cargo emits backslash-separated manifest_path
    // values. The comparison must use Path-based equivalence so platform
    // separators line up.
    let m = Metadata::from_value(serde_json::json!({
        "workspace_root": "C:\\workspace",
        "target_directory": "C:\\workspace\\target",
        "workspace_members": ["root-pkg 0.1.0 (path+file:///C:/workspace)"],
        "packages": [
            {
                "name": "root-pkg",
                "version": "0.1.0",
                "id": "root-pkg 0.1.0 (path+file:///C:/workspace)",
                "edition": "2021",
                "manifest_path": "C:\\workspace\\Cargo.toml",
                "dependencies": [],
                "targets": []
            }
        ]
    }));
    let root = m.root_package().expect("should find root package");
    assert_eq!(root.name(), "root-pkg");
}

#[test]
fn metadata_root_package_uses_path_equivalence() {
    // TASK-0952: trailing slash on workspace_root should not break the join.
    let m = Metadata::from_value(serde_json::json!({
        "workspace_root": "/workspace/",
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
    let root = m
        .root_package()
        .expect("should find root package via Path equivalence");
    assert_eq!(root.name(), "root-pkg");
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

    #[test]
    fn package_id_accessor() {
        let m = Metadata::from_value(sample_metadata());
        let p = test_pkg_a(&m);
        assert_eq!(p.id(), "pkg-a 0.1.0 (path+file:///workspace/pkg-a)");
    }

    #[test]
    fn target_edition_present() {
        let m = Metadata::from_value(serde_json::json!({
            "workspace_root": "/workspace",
            "target_directory": "/workspace/target",
            "workspace_members": ["pkg 0.1.0 (path+file:///workspace/pkg)"],
            "packages": [{
                "name": "pkg",
                "version": "0.1.0",
                "id": "pkg 0.1.0 (path+file:///workspace/pkg)",
                "manifest_path": "/workspace/pkg/Cargo.toml",
                "dependencies": [],
                "targets": [{
                    "name": "pkg",
                    "kind": ["lib"],
                    "src_path": "/workspace/pkg/src/lib.rs",
                    "edition": "2021"
                }]
            }]
        }));
        let pkg = m.package_by_name("pkg").unwrap();
        let lib = pkg.lib_target().unwrap();
        assert_eq!(lib.edition(), Some("2021"));
    }

    #[test]
    fn target_edition_absent() {
        let m = Metadata::from_value(sample_metadata());
        let p = test_pkg_a(&m);
        let lib = p.lib_target().unwrap();
        assert!(lib.edition().is_none());
    }

    #[test]
    fn target_doc_path_present() {
        let m = Metadata::from_value(serde_json::json!({
            "workspace_root": "/workspace",
            "target_directory": "/workspace/target",
            "workspace_members": ["pkg 0.1.0 (path+file:///workspace/pkg)"],
            "packages": [{
                "name": "pkg",
                "version": "0.1.0",
                "id": "pkg 0.1.0 (path+file:///workspace/pkg)",
                "manifest_path": "/workspace/pkg/Cargo.toml",
                "dependencies": [],
                "targets": [{
                    "name": "pkg",
                    "kind": ["lib"],
                    "src_path": "/workspace/pkg/src/lib.rs",
                    "doc_path": "/workspace/pkg/src/lib.rs"
                }]
            }]
        }));
        let pkg = m.package_by_name("pkg").unwrap();
        let lib = pkg.lib_target().unwrap();
        assert_eq!(lib.doc_path(), Some("/workspace/pkg/src/lib.rs"));
    }

    #[test]
    fn target_doc_path_absent() {
        let m = Metadata::from_value(sample_metadata());
        let p = test_pkg_a(&m);
        let lib = p.lib_target().unwrap();
        assert!(lib.doc_path().is_none());
    }

    #[test]
    fn dependency_with_rename() {
        let m = Metadata::from_value(serde_json::json!({
            "workspace_root": "/workspace",
            "target_directory": "/workspace/target",
            "workspace_members": ["pkg 0.1.0 (path+file:///workspace/pkg)"],
            "packages": [{
                "name": "pkg",
                "version": "0.1.0",
                "id": "pkg 0.1.0 (path+file:///workspace/pkg)",
                "manifest_path": "/workspace/pkg/Cargo.toml",
                "dependencies": [{
                    "name": "serde",
                    "req": "^1.0",
                    "rename": "my_serde",
                    "source": "registry+https://github.com/rust-lang/crates.io-index"
                }],
                "targets": []
            }]
        }));
        let pkg = m.package_by_name("pkg").unwrap();
        let dep = pkg.all_dependencies().next().unwrap();
        assert_eq!(dep.rename(), Some("my_serde"));
        assert!(dep.source().is_some());
    }

    #[test]
    fn dependency_with_target_platform() {
        let m = Metadata::from_value(serde_json::json!({
            "workspace_root": "/workspace",
            "target_directory": "/workspace/target",
            "workspace_members": ["pkg 0.1.0 (path+file:///workspace/pkg)"],
            "packages": [{
                "name": "pkg",
                "version": "0.1.0",
                "id": "pkg 0.1.0 (path+file:///workspace/pkg)",
                "manifest_path": "/workspace/pkg/Cargo.toml",
                "dependencies": [{
                    "name": "winapi",
                    "req": "^0.3",
                    "target": "cfg(windows)"
                }],
                "targets": []
            }]
        }));
        let pkg = m.package_by_name("pkg").unwrap();
        let dep = pkg.all_dependencies().next().unwrap();
        assert_eq!(dep.target(), Some("cfg(windows)"));
    }

    #[test]
    fn metadata_multiple_workspace_members() {
        let m = Metadata::from_value(serde_json::json!({
            "workspace_root": "/workspace",
            "target_directory": "/workspace/target",
            "workspace_members": [
                "pkg-a 0.1.0 (path+file:///workspace/pkg-a)",
                "pkg-b 0.2.0 (path+file:///workspace/pkg-b)"
            ],
            "workspace_default_members": [
                "pkg-a 0.1.0 (path+file:///workspace/pkg-a)"
            ],
            "packages": [
                {
                    "name": "pkg-a",
                    "version": "0.1.0",
                    "id": "pkg-a 0.1.0 (path+file:///workspace/pkg-a)",
                    "edition": "2021",
                    "manifest_path": "/workspace/pkg-a/Cargo.toml",
                    "dependencies": [],
                    "targets": []
                },
                {
                    "name": "pkg-b",
                    "version": "0.2.0",
                    "id": "pkg-b 0.2.0 (path+file:///workspace/pkg-b)",
                    "edition": "2021",
                    "manifest_path": "/workspace/pkg-b/Cargo.toml",
                    "dependencies": [],
                    "targets": []
                },
                {
                    "name": "external",
                    "version": "1.0.0",
                    "id": "external 1.0.0 (registry+https://crates.io)",
                    "edition": "2018",
                    "manifest_path": "/cargo/registry/external-1.0.0/Cargo.toml",
                    "dependencies": [],
                    "targets": []
                }
            ]
        }));
        let members: Vec<&str> = m.members().map(|p| p.name()).collect();
        assert_eq!(members, vec!["pkg-a", "pkg-b"]);

        let defaults: Vec<&str> = m.default_members().map(|p| p.name()).collect();
        assert_eq!(defaults, vec!["pkg-a"]);

        assert!(m.package_by_name("pkg-b").unwrap().is_member());
        assert!(!m.package_by_name("pkg-b").unwrap().is_default_member());
        assert!(!m.package_by_name("external").unwrap().is_member());
    }

    #[test]
    fn metadata_package_with_all_target_types() {
        let m = Metadata::from_value(serde_json::json!({
            "workspace_root": "/workspace",
            "target_directory": "/workspace/target",
            "workspace_members": ["pkg 0.1.0 (path+file:///workspace/pkg)"],
            "packages": [{
                "name": "pkg",
                "version": "0.1.0",
                "id": "pkg 0.1.0 (path+file:///workspace/pkg)",
                "manifest_path": "/workspace/pkg/Cargo.toml",
                "dependencies": [],
                "targets": [
                    {"name": "pkg", "kind": ["lib"], "src_path": "/workspace/pkg/src/lib.rs"},
                    {"name": "cli", "kind": ["bin"], "src_path": "/workspace/pkg/src/main.rs"},
                    {"name": "integration", "kind": ["test"], "src_path": "/workspace/pkg/tests/integration.rs"},
                    {"name": "demo", "kind": ["example"], "src_path": "/workspace/pkg/examples/demo.rs"},
                    {"name": "perf", "kind": ["bench"], "src_path": "/workspace/pkg/benches/perf.rs"}
                ]
            }]
        }));
        let pkg = m.package_by_name("pkg").unwrap();
        assert!(pkg.lib_target().is_some());
        assert_eq!(pkg.bin_targets().count(), 1);
        assert_eq!(pkg.test_targets().count(), 1);
        assert_eq!(pkg.example_targets().count(), 1);
        assert_eq!(pkg.bench_targets().count(), 1);

        let test = pkg.test_targets().next().unwrap();
        assert!(test.is_test());
        assert!(!test.is_lib());

        let example = pkg.example_targets().next().unwrap();
        assert!(example.is_example());

        let bench = pkg.bench_targets().next().unwrap();
        assert!(bench.is_bench());
    }

    #[test]
    fn metadata_schema_has_expected_fields() {
        use ops_extension::DataProvider;
        let schema = MetadataProvider.schema();
        assert!(!schema.fields.is_empty());
        let field_names: Vec<&str> = schema.fields.iter().map(|f| f.name).collect();
        assert!(field_names.contains(&"workspace_root"));
        assert!(field_names.contains(&"packages"));
        assert!(field_names.contains(&"members"));
    }

    #[test]
    fn check_metadata_output_success() {
        use std::process::Output;
        let output = Output {
            status: std::process::ExitStatus::default(),
            stdout: vec![],
            stderr: vec![],
        };
        // ExitStatus::default() is success (code 0) on unix
        #[cfg(unix)]
        assert!(check_metadata_output(&output).is_ok());
    }

    /// PATTERN-1 / TASK-1099: non-zero exit codes must appear in the
    /// error string so a real cargo failure (exit 1, exit 101 panic) is
    /// distinguishable from infrastructure (SIGKILL/OOM, surfaced as
    /// `signal`).
    #[cfg(unix)]
    #[test]
    fn check_metadata_output_failure_includes_exit_code() {
        use std::os::unix::process::ExitStatusExt;
        use std::process::Output;
        // exit code 101 (panic-style)
        let output = Output {
            status: std::process::ExitStatus::from_raw(101 << 8),
            stdout: vec![],
            stderr: b"thread 'main' panicked".to_vec(),
        };
        let err = check_metadata_output(&output).expect_err("non-zero must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("status 101"),
            "exit code 101 must appear in error: {msg}"
        );
        assert!(
            msg.contains("panicked"),
            "stderr tail must remain in error: {msg}"
        );
    }

    /// PATTERN-1 / TASK-1099: a None exit (signal kill, e.g. OOM)
    /// surfaces as `signal` rather than the same string as a normal
    /// non-zero exit.
    #[cfg(unix)]
    #[test]
    fn check_metadata_output_failure_signal_kill_says_signal() {
        use std::os::unix::process::ExitStatusExt;
        use std::process::Output;
        // signal 9 (SIGKILL) → exit_code() returns None
        let output = Output {
            status: std::process::ExitStatus::from_raw(9),
            stdout: vec![],
            stderr: b"".to_vec(),
        };
        let err = check_metadata_output(&output).expect_err("signal must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("signal") || msg.contains("None"),
            "signal-kill case must be named in error: {msg}"
        );
    }

    /// PATTERN-1 / TASK-1059: `cargo metadata` must run with `--locked`
    /// so the read-only ingestor cannot mutate Cargo.lock. The actual
    /// subprocess invocation goes through `run_cargo`; pin the arg list
    /// at the source-of-truth here so a future refactor cannot silently
    /// drop the flag.
    #[test]
    fn run_cargo_metadata_arg_list_includes_locked() {
        // Read the current source of `run_cargo_metadata` and verify
        // the static arg list includes `--locked`. This is a coarse
        // pin but it withstands moving the function body around without
        // requiring a fake `cargo` on PATH.
        let src = include_str!("lib.rs");
        // Find the `run_cargo_metadata` arg slice and assert --locked
        // appears between `metadata` and the closing bracket.
        let needle = "[\"metadata\", \"--format-version\", \"1\", \"--locked\"]";
        assert!(
            src.contains(needle),
            "run_cargo_metadata arg list must include --locked (TASK-1059); src missing: {needle}"
        );
    }

    #[test]
    fn metadata_package_by_id_returns_none_for_missing() {
        let m = Metadata::from_value(sample_metadata());
        assert!(m
            .package_by_id("nonexistent 0.0.0 (path+file:///nowhere)")
            .is_none());
    }

    /// ERR-1 / TASK-0599: `metadata_raw` is a singleton invariant. If a
    /// future ingest path (re-collect without truncate, schema-version row)
    /// inserts more than one row, `query_metadata_raw` must surface a
    /// clear error rather than silently picking an arbitrary row via
    /// `LIMIT 1`.
    #[test]
    fn query_metadata_raw_errors_on_multiple_rows() {
        let db = ops_duckdb::DuckDb::open_in_memory().expect("open in-memory");
        {
            let conn = db.lock().expect("lock");
            conn.execute_batch(
                "CREATE TABLE metadata_raw (workspace_root VARCHAR, payload INTEGER);
                 INSERT INTO metadata_raw VALUES ('/a', 1), ('/b', 2);",
            )
            .expect("seed");
        }
        let err = super::query_metadata_raw(&db).expect_err("multi-row must fail");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("exactly one row") || msg.contains("found 2"),
            "got: {msg}"
        );
    }

    /// ERR-1 / TASK-0599: companion to the multi-row test — single-row
    /// metadata_raw flows through unchanged.
    #[test]
    fn query_metadata_raw_succeeds_on_single_row() {
        let db = ops_duckdb::DuckDb::open_in_memory().expect("open in-memory");
        {
            let conn = db.lock().expect("lock");
            conn.execute_batch(
                "CREATE TABLE metadata_raw (workspace_root VARCHAR, payload INTEGER);
                 INSERT INTO metadata_raw VALUES ('/a', 1);",
            )
            .expect("seed");
        }
        let v = super::query_metadata_raw(&db).expect("single-row must succeed");
        assert_eq!(v["workspace_root"], "/a");
    }

    /// ERR-1 / TASK-1034: oversized payloads must fail fast with a
    /// clear error rather than risking an OOM in `ops about`. The cap
    /// is configurable via `OPS_METADATA_MAX_BYTES`; this test drives
    /// the cap directly to avoid mutating process-global env.
    #[test]
    fn query_metadata_raw_errors_when_payload_exceeds_cap() {
        let db = ops_duckdb::DuckDb::open_in_memory().expect("open in-memory");
        {
            let conn = db.lock().expect("lock");
            // A row whose to_json serialisation comfortably exceeds 32 bytes.
            conn.execute_batch(
                "CREATE TABLE metadata_raw (workspace_root VARCHAR, payload VARCHAR);
                 INSERT INTO metadata_raw VALUES \
                 ('/workspace', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa');",
            )
            .expect("seed");
        }
        let err = super::query_metadata_raw_with_cap(&db, 32).expect_err("oversized must fail");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("exceeds") && msg.contains("byte cap"),
            "got: {msg}"
        );
        assert!(msg.contains(super::METADATA_MAX_BYTES_ENV), "got: {msg}");
    }

    /// ERR-1 / TASK-1034: payloads at or under the cap parse normally.
    #[test]
    fn query_metadata_raw_succeeds_when_payload_within_cap() {
        let db = ops_duckdb::DuckDb::open_in_memory().expect("open in-memory");
        {
            let conn = db.lock().expect("lock");
            conn.execute_batch(
                "CREATE TABLE metadata_raw (workspace_root VARCHAR, payload INTEGER);
                 INSERT INTO metadata_raw VALUES ('/workspace', 1);",
            )
            .expect("seed");
        }
        let v = super::query_metadata_raw_with_cap(&db, super::METADATA_MAX_BYTES_DEFAULT)
            .expect("under-cap payload should parse");
        assert_eq!(v["workspace_root"], "/workspace");
    }

    #[test]
    fn metadata_missing_packages_key() {
        let m = Metadata::from_value(serde_json::json!({
            "workspace_root": "/workspace",
            "target_directory": "/workspace/target"
        }));
        assert_eq!(m.packages().count(), 0);
        assert_eq!(m.members().count(), 0);
    }

    #[test]
    fn target_required_features_empty() {
        let m = Metadata::from_value(sample_metadata());
        let p = test_pkg_a(&m);
        let lib = p.lib_target().unwrap();
        assert_eq!(lib.required_features().count(), 0);
    }

    /// PATTERN-1 / TASK-1100: Duplicate package ids in `inner["packages"]`
    /// must emit a single `tracing::warn!` and the index must keep the
    /// first-seen entry (first-write-wins) rather than silently overwriting.
    #[test]
    fn metadata_package_index_by_id_warns_on_duplicate_id() {
        use std::sync::{Arc as StdArc, Mutex as StdMutex};
        use tracing_subscriber::fmt::MakeWriter;

        #[derive(Clone, Default)]
        struct BufWriter(StdArc<StdMutex<Vec<u8>>>);
        impl std::io::Write for BufWriter {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(b);
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        impl<'a> MakeWriter<'a> for BufWriter {
            type Writer = BufWriter;
            fn make_writer(&'a self) -> Self::Writer {
                self.clone()
            }
        }

        let dup_id = "dup-pkg 0.1.0 (path+file:///workspace/dup)";
        let value = serde_json::json!({
            "workspace_root": "/workspace",
            "target_directory": "/workspace/target",
            "workspace_members": [],
            "workspace_default_members": [],
            "packages": [
                {
                    "name": "dup-pkg",
                    "version": "0.1.0",
                    "id": dup_id,
                    "edition": "2021",
                    "manifest_path": "/workspace/dup/a/Cargo.toml",
                    "dependencies": [],
                    "targets": [],
                    "description": "first"
                },
                {
                    "name": "dup-pkg",
                    "version": "0.1.0",
                    "id": dup_id,
                    "edition": "2021",
                    "manifest_path": "/workspace/dup/b/Cargo.toml",
                    "dependencies": [],
                    "targets": [],
                    "description": "second"
                }
            ]
        });

        let buf = BufWriter::default();
        let captured = buf.0.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buf)
            .with_max_level(tracing::Level::WARN)
            .with_ansi(false)
            .finish();

        let m = Metadata::from_value(value);
        let pkg = tracing::subscriber::with_default(subscriber, || {
            // Force lazy index construction.
            m.package_by_id(dup_id)
        })
        .expect("first-seen entry must be present");

        // First-write-wins: manifest_path of the first package, not the second.
        assert_eq!(
            pkg.manifest_path(),
            "/workspace/dup/a/Cargo.toml",
            "first-seen entry must win on duplicate id"
        );

        let logs = String::from_utf8(captured.lock().unwrap().clone()).unwrap();
        let warn_lines: Vec<&str> = logs
            .lines()
            .filter(|l| l.contains("duplicate package id"))
            .collect();
        assert_eq!(
            warn_lines.len(),
            1,
            "expected exactly one warn line for the single duplicate, got logs: {logs}"
        );
        assert!(
            warn_lines[0].contains(dup_id),
            "warn line should name the duplicate id, got: {}",
            warn_lines[0]
        );
    }

    /// PATTERN-1 / TASK-1019: Duplicate package names in `inner["packages"]`
    /// (e.g. the same crate resolved at two versions) must emit a single
    /// `tracing::warn!` and the index must keep the first-seen entry rather
    /// than silently overwriting (last-write-wins). Consumers calling
    /// `package_by_name` then get a deterministic, observable answer; for
    /// version disambiguation they must use `package_by_id`.
    #[test]
    fn metadata_package_index_by_name_warns_on_duplicate_name() {
        use std::sync::{Arc as StdArc, Mutex as StdMutex};
        use tracing_subscriber::fmt::MakeWriter;

        #[derive(Clone, Default)]
        struct BufWriter(StdArc<StdMutex<Vec<u8>>>);
        impl std::io::Write for BufWriter {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(b);
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        impl<'a> MakeWriter<'a> for BufWriter {
            type Writer = BufWriter;
            fn make_writer(&'a self) -> Self::Writer {
                self.clone()
            }
        }

        let id_v1 = "serde 1.0.0 (registry+https://github.com/rust-lang/crates.io-index)";
        let id_v0 = "serde 0.9.0 (registry+https://github.com/rust-lang/crates.io-index)";
        let value = serde_json::json!({
            "workspace_root": "/workspace",
            "target_directory": "/workspace/target",
            "workspace_members": [],
            "workspace_default_members": [],
            "packages": [
                {
                    "name": "serde",
                    "version": "1.0.0",
                    "id": id_v1,
                    "edition": "2021",
                    "manifest_path": "/cache/serde-1.0.0/Cargo.toml",
                    "dependencies": [],
                    "targets": []
                },
                {
                    "name": "serde",
                    "version": "0.9.0",
                    "id": id_v0,
                    "edition": "2018",
                    "manifest_path": "/cache/serde-0.9.0/Cargo.toml",
                    "dependencies": [],
                    "targets": []
                }
            ]
        });

        let buf = BufWriter::default();
        let captured = buf.0.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buf)
            .with_max_level(tracing::Level::WARN)
            .with_ansi(false)
            .finish();

        let m = Metadata::from_value(value);
        let pkg = tracing::subscriber::with_default(subscriber, || {
            // Force lazy index construction.
            m.package_by_name("serde")
        })
        .expect("first-seen entry must be present");

        // First-write-wins: version of the first package, not the second.
        assert_eq!(
            pkg.version(),
            "1.0.0",
            "first-seen entry must win on duplicate name"
        );
        assert_eq!(
            pkg.manifest_path(),
            "/cache/serde-1.0.0/Cargo.toml",
            "first-seen entry must win on duplicate name"
        );

        // Both packages still reachable via package_by_id (the disambiguating
        // accessor). This documents the recommended workaround for callers
        // that hit this collision.
        assert_eq!(m.package_by_id(id_v1).expect("v1 by id").version(), "1.0.0");
        assert_eq!(m.package_by_id(id_v0).expect("v0 by id").version(), "0.9.0");

        let logs = String::from_utf8(captured.lock().unwrap().clone()).unwrap();
        let warn_lines: Vec<&str> = logs
            .lines()
            .filter(|l| l.contains("duplicate package name"))
            .collect();
        assert_eq!(
            warn_lines.len(),
            1,
            "expected exactly one warn line for the single duplicate, got logs: {logs}"
        );
        assert!(
            warn_lines[0].contains("serde"),
            "warn line should name the duplicate package, got: {}",
            warn_lines[0]
        );
    }
}
