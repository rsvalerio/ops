---
id: TASK-2495
title: 'API-15: ops-cargo-toml library declares no rust-version (MSRV)'
status: To Do
assignee: []
created_date: '2026-10-10 15:31'
updated_date: '2026-10-10 21:12'
labels:
  - code-review
  - api
dependencies: []
parent_task_id: 'TASK-2608'
modified_files:
  - extensions-rust/cargo-toml/Cargo.toml
priority: low
ordinal: 1000
dedup_key: 'API-15:extensions-rust/cargo-toml/Cargo.toml:ops-cargo-toml'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/Cargo.toml:1`

**What**: The crate's [package] section inherits version/edition/license from the workspace but does not declare `rust-version`, and the workspace's `[workspace.package]` rust-version = "1.97" is only inherited by members that opt in with `rust-version.workspace = true` (only crates/cli currently does). The library therefore has no declared MSRV of its own.

**Why it matters**: API-15 — a library with no declared MSRV has an accidental one that changes with whatever the maintainer happened to compile with. Adding `rust-version.workspace = true` pins this crate to the workspace MSRV (1.97) and keeps it tracking workspace-wide bumps. Matches the finding filed for ops-theme (TASK-2450); this crate has the same shape.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Cargo.toml [package] contains rust-version.workspace = true (or an explicit rust-version matching the workspace value)
- [ ] #2 cargo check succeeds and cargo metadata reports the crate's rust_version as the workspace MSRV
<!-- AC:END -->
