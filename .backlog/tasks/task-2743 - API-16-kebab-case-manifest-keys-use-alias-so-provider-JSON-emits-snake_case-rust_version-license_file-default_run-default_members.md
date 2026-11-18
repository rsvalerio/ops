---
id: TASK-2743
title: 'API-16: kebab-case manifest keys use alias, so provider JSON emits snake_case rust_version / license_file / default_run / default_members'
status: Done
assignee: []
created_date: '2026-11-18 18:38'
updated_date: '2026-11-18 22:30'
labels:
  - code-review-rust
  - api-design
dependencies: []
parent_task_id: 'TASK-2850'
modified_files:
  - extensions-rust/cargo-toml/src/types.rs
priority: low
ordinal: 1000
dedup_key: 'API-16:extensions-rust/cargo-toml/src/types.rs:Package'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/types.rs:130` (`Package::rust_version`), `:161` (`Package::license_file`), `:173` (`Package::default_run`), `:322` (`Workspace::default_members`), `:364` (`WorkspacePackage::rust_version`), `:379` (`WorkspacePackage::license_file`)

**What**: These fields map Cargo's kebab-case keys with `#[serde(alias = "...")]` only. Deserialisation accepts `rust-version`, but `Serialize` writes the Rust name, so `CargoTomlProvider::provide` emits `rust_version`, `license_file`, `default_run`, `default_members`. The crate already fixed this exact problem for `dev-dependencies` / `build-dependencies` (see the doc comment at `types.rs:52`: `rename` plus a snake_case `alias`), but not for these fields. Also, `Package::default_run` is documented as "Path to the main source file"; it is the name of the default binary for `cargo run`.

**Why it matters**: JSON consumers reading Cargo's spelling (`rust-version`) silently get nothing, and the provider's key spelling is inconsistent across fields of the same manifest. The serde convention in the Rust API Guidelines (C-SERDE) is a stable, format-faithful field naming.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each kebab-case Cargo key is mapped with rename = "<kebab>" plus alias = "<snake>", matching dev-dependencies
- [x] #2 A round-trip test asserts the serialized JSON keys use Cargo's spelling (rust-version, license-file, default-run, default-members)
- [x] #3 Package::default_run's doc describes the default binary name

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.12 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
