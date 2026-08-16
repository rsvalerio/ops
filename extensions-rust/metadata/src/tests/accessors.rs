//! Accessor coverage on top of the shared `sample_metadata` fixture.
//!
//! ARCH-1 / TASK-1545: split out from the legacy `tests.rs`.

use crate::test_support::{pkg, sample_metadata, test_pkg_a, test_pkg_serde, workspace};
use crate::{DependencyKind, Metadata};

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
    // A root package's manifest sits at the workspace root, not in a
    // subdirectory — which is the whole point of the lookup under test.
    let m = workspace()
        .member(
            pkg("root-pkg", "0.1.0")
                .id("root-pkg 0.1.0 (path+file:///workspace)")
                .manifest_path("/workspace/Cargo.toml")
                .edition("2021"),
        )
        .metadata();
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
    let m = workspace().metadata();
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

#[test]
fn package_id_accessor() {
    let m = Metadata::from_value(sample_metadata());
    let p = test_pkg_a(&m);
    assert_eq!(p.id(), "pkg-a 0.1.0 (path+file:///workspace/pkg-a)");
}

#[test]
fn target_edition_absent() {
    let m = Metadata::from_value(sample_metadata());
    let p = test_pkg_a(&m);
    let lib = p.lib_target().unwrap();
    assert!(lib.edition().is_none());
}

#[test]
fn target_doc_path_absent() {
    let m = Metadata::from_value(sample_metadata());
    let p = test_pkg_a(&m);
    let lib = p.lib_target().unwrap();
    assert!(lib.doc_path().is_none());
}

#[test]
fn target_required_features_empty() {
    let m = Metadata::from_value(sample_metadata());
    let p = test_pkg_a(&m);
    let lib = p.lib_target().unwrap();
    assert_eq!(lib.required_features().count(), 0);
}

#[test]
fn metadata_package_by_id_returns_none_for_missing() {
    let m = Metadata::from_value(sample_metadata());
    assert!(m
        .package_by_id("nonexistent 0.0.0 (path+file:///nowhere)")
        .is_none());
}

/// PERF-3 / TASK-1248 AC #3: two consecutive `metadata_max_bytes()`
/// calls return the same value — the OnceLock-snapshotted cap stays
/// stable for the rest of the process.
#[test]
fn metadata_max_bytes_is_memoised() {
    let a = crate::metadata_max_bytes();
    let b = crate::metadata_max_bytes();
    assert_eq!(
        a, b,
        "cached metadata_max_bytes must not drift between calls"
    );
}
