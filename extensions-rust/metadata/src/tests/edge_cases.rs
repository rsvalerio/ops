//! Minimal-field and missing-optional edge case coverage.
//!
//! ARCH-1 / TASK-1545: split out from the legacy `tests.rs`.
//! DUP-4 / TASK-1540: the cargo-metadata skeleton comes from
//! `crate::test_support`. These tests are mostly *about* absent fields, so the
//! fixtures there emit only what a test sets — a package built with no
//! `.edition(...)` really has no `edition` key.

use crate::test_support::{pkg, workspace};
use crate::Metadata;

#[test]
fn metadata_build_directory_none_when_missing() {
    let m = workspace().metadata();
    assert!(m.build_directory().is_none());
}

#[test]
fn metadata_empty_workspace_members() {
    let m = workspace().metadata();
    let members: Vec<_> = m.members().collect();
    assert!(members.is_empty(), "empty workspace should have no members");
}

#[test]
fn metadata_package_no_targets() {
    let m = workspace()
        .member(pkg("empty-pkg", "0.1.0").edition("2021"))
        .metadata();
    let pkg = m.package_by_name("empty-pkg").expect("should find package");
    assert_eq!(pkg.targets().count(), 0, "package should have no targets");
    assert!(pkg.lib_target().is_none());
    assert_eq!(pkg.bin_targets().count(), 0);
}

#[test]
fn metadata_package_no_dependencies() {
    let m = workspace()
        .member(pkg("no-deps", "0.1.0").edition("2021"))
        .metadata();
    let pkg = m.package_by_name("no-deps").expect("should find package");
    assert_eq!(pkg.all_dependencies().count(), 0);
    assert_eq!(pkg.dependencies().count(), 0);
    assert_eq!(pkg.dev_dependencies().count(), 0);
    assert_eq!(pkg.build_dependencies().count(), 0);
}

#[test]
fn metadata_package_missing_optional_fields() {
    // No `.edition(...)`, no license/repository/description: the fixture emits
    // none of them, which is exactly what this test asserts about.
    let m = workspace().member(pkg("minimal", "0.1.0")).metadata();
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
    let m = workspace()
        .member(pkg("pkg", "0.1.0").deps(serde_json::json!([
            {
                "name": "minimal-dep",
                "req": "^1.0"
            }
        ])))
        .metadata();
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
    let m = workspace()
        .member(pkg("pkg", "0.1.0").targets(serde_json::json!([{
            "name": "pkg",
            "kind": ["lib"],
            "src_path": "/workspace/pkg/src/lib.rs",
            "edition": "2021"
        }])))
        .metadata();
    let pkg = m.package_by_name("pkg").unwrap();
    let lib = pkg.lib_target().unwrap();
    assert_eq!(lib.edition(), Some("2021"));
}

#[test]
fn target_doc_path_present() {
    let m = workspace()
        .member(pkg("pkg", "0.1.0").targets(serde_json::json!([{
            "name": "pkg",
            "kind": ["lib"],
            "src_path": "/workspace/pkg/src/lib.rs",
            "doc_path": "/workspace/pkg/src/lib.rs"
        }])))
        .metadata();
    let pkg = m.package_by_name("pkg").unwrap();
    let lib = pkg.lib_target().unwrap();
    assert_eq!(lib.doc_path(), Some("/workspace/pkg/src/lib.rs"));
}

#[test]
fn dependency_with_rename() {
    let m = workspace()
        .member(pkg("pkg", "0.1.0").deps(serde_json::json!([{
            "name": "serde",
            "req": "^1.0",
            "rename": "my_serde",
            "source": "registry+https://github.com/rust-lang/crates.io-index"
        }])))
        .metadata();
    let pkg = m.package_by_name("pkg").unwrap();
    let dep = pkg.all_dependencies().next().unwrap();
    assert_eq!(dep.rename(), Some("my_serde"));
    assert!(dep.source().is_some());
}

#[test]
fn dependency_with_target_platform() {
    let m = workspace()
        .member(pkg("pkg", "0.1.0").deps(serde_json::json!([{
            "name": "winapi",
            "req": "^0.3",
            "target": "cfg(windows)"
        }])))
        .metadata();
    let pkg = m.package_by_name("pkg").unwrap();
    let dep = pkg.all_dependencies().next().unwrap();
    assert_eq!(dep.target(), Some("cfg(windows)"));
}

#[test]
fn metadata_multiple_workspace_members() {
    let m = workspace()
        .member(pkg("pkg-a", "0.1.0").edition("2021"))
        .member(pkg("pkg-b", "0.2.0").edition("2021"))
        .external(
            pkg("external", "1.0.0")
                .id("external 1.0.0 (registry+https://crates.io)")
                .manifest_path("/cargo/registry/external-1.0.0/Cargo.toml")
                .edition("2018"),
        )
        .default_members(&["pkg-a"])
        .metadata();
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
    let m = workspace()
        .member(pkg("pkg", "0.1.0").targets(serde_json::json!([
            {"name": "pkg", "kind": ["lib"], "src_path": "/workspace/pkg/src/lib.rs"},
            {"name": "cli", "kind": ["bin"], "src_path": "/workspace/pkg/src/main.rs"},
            {"name": "integration", "kind": ["test"], "src_path": "/workspace/pkg/tests/integration.rs"},
            {"name": "demo", "kind": ["example"], "src_path": "/workspace/pkg/examples/demo.rs"},
            {"name": "perf", "kind": ["bench"], "src_path": "/workspace/pkg/benches/perf.rs"}
        ])))
        .metadata();
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
    //
    // Written out rather than built from `test_support`: the fixture derives
    // `target_directory` by joining with `/`, and a Windows-separator setter
    // would be dead code on every other platform.
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
    let m = workspace()
        .root("/workspace/")
        .member(
            pkg("root-pkg", "0.1.0")
                .id("root-pkg 0.1.0 (path+file:///workspace)")
                .manifest_path("/workspace/Cargo.toml")
                .edition("2021"),
        )
        .metadata();
    let root = m
        .root_package()
        .expect("should find root package via Path equivalence");
    assert_eq!(root.name(), "root-pkg");
}

#[test]
fn metadata_missing_packages_key() {
    // Written out rather than built from `test_support`: the fixture always
    // emits `packages`, and the absence of that key is the subject here.
    let m = Metadata::from_value(serde_json::json!({
        "workspace_root": "/workspace",
        "target_directory": "/workspace/target"
    }));
    assert_eq!(m.packages().count(), 0);
    assert_eq!(m.members().count(), 0);
}
