//! Minimal-field and missing-optional edge case coverage.
//!
//! ARCH-1 / TASK-1545: split out from the legacy `tests.rs`.

use crate::Metadata;

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
fn metadata_missing_packages_key() {
    let m = Metadata::from_value(serde_json::json!({
        "workspace_root": "/workspace",
        "target_directory": "/workspace/target"
    }));
    assert_eq!(m.packages().count(), 0);
    assert_eq!(m.members().count(), 0);
}
