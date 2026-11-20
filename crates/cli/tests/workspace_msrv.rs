//! Every workspace member inherits the workspace MSRV.
//!
//! Each member is meant to declare `rust-version.workspace = true`, but a
//! new crate created without the key still builds, lints and passes
//! `ops verify`: cargo just stops rejecting older toolchains for it. This
//! test reads what cargo actually resolves (`cargo metadata`), so any form of
//! drift fails it — a missing key, or a literal that differs from the root
//! `[workspace.package] rust-version`.

// An integration-test target is its own crate (docs/clippy.md layer 2): the
// helpers below are fixtures where a panic is the desired failure mode.
#![allow(clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/cli sits two levels below the workspace root")
        .to_path_buf()
}

fn root_rust_version(root: &Path) -> String {
    let text = std::fs::read_to_string(root.join("Cargo.toml")).expect("read root Cargo.toml");
    let manifest: toml::Table = text.parse().expect("parse root Cargo.toml");
    manifest
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("rust-version"))
        .and_then(toml::Value::as_str)
        .expect("root Cargo.toml declares [workspace.package] rust-version")
        .to_string()
}

#[test]
fn every_member_resolves_the_workspace_rust_version() {
    let root = workspace_root();
    let expected = root_rust_version(&root);
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = ops_core::subprocess::output_locked(
        Command::new(cargo)
            .args(["metadata", "--no-deps", "--format-version", "1", "--offline"])
            .current_dir(&root),
    )
    .expect("spawn cargo metadata");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("cargo metadata prints JSON");
    let packages = metadata["packages"].as_array().expect("metadata lists packages");
    assert!(!packages.is_empty(), "cargo metadata listed no workspace members");

    let drift: Vec<String> = packages
        .iter()
        .filter_map(|pkg| {
            let name = pkg["name"].as_str().unwrap_or("<unnamed>");
            match pkg["rust_version"].as_str() {
                Some(v) if v == expected => None,
                Some(v) => Some(format!("{name}: rust-version {v:?}")),
                None => Some(format!("{name}: no rust-version")),
            }
        })
        .collect();
    assert!(
        drift.is_empty(),
        "workspace members that do not inherit rust-version {expected:?} \
         (add `rust-version.workspace = true` under [package]):\n  {}",
        drift.join("\n  ")
    );
}
