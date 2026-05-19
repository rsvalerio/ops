//! Shared cargo-metadata JSON fixtures used by the split test modules.
//!
//! ARCH-1 / TASK-1545: lifted out of the legacy 1280-line `tests.rs` so
//! every sibling test module reads the fixture from one place.

use crate::{Metadata, Package};

pub(super) fn sample_metadata() -> serde_json::Value {
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

pub(super) fn test_pkg_a(metadata: &Metadata) -> Package<'_> {
    metadata.package_by_name("pkg-a").expect("fixture: pkg-a")
}

pub(super) fn test_pkg_serde(metadata: &Metadata) -> Package<'_> {
    metadata.package_by_name("serde").expect("fixture: serde")
}
