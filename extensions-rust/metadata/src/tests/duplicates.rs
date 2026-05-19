//! Duplicate-id / duplicate-name warning tests.
//!
//! ARCH-1 / TASK-1545: split out from the legacy `tests.rs`.
//! DUP-3 / TASK-1538: both tests share the `TracingBuf` harness from
//! `ops_about::test_support` so a future harness change lives in one place.

use crate::Metadata;
use ops_about::test_support::TracingBuf;

/// PATTERN-1 / TASK-1100: Duplicate package ids in `inner["packages"]`
/// must emit a single `tracing::warn!` and the index must keep the
/// first-seen entry (first-write-wins) rather than silently overwriting.
#[test]
fn metadata_package_index_by_id_warns_on_duplicate_id() {
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

    let buf = TracingBuf::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(buf.clone())
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

    let logs = buf.captured();
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

    let buf = TracingBuf::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(buf.clone())
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

    let logs = buf.captured();
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
