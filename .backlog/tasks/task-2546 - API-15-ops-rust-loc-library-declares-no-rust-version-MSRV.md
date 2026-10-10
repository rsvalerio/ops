---
id: TASK-2546
title: 'API-15: ops-rust-loc library declares no rust-version (MSRV)'
status: Done
assignee: []
created_date: '2026-10-10 15:38'
updated_date: '2026-10-10 22:26'
labels:
  - code-review
  - api
dependencies: []
parent_task_id: 'TASK-2608'
modified_files:
  - extensions-rust/loc/Cargo.toml
priority: low
ordinal: 1000
dedup_key: 'API-15:extensions-rust/loc/Cargo.toml:ops-rust-loc'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/loc/Cargo.toml`

**What**: The crate's `[package]` section inherits `version`, `edition` and `license` from `[workspace.package]` but not `rust-version`, and the workspace root defines `rust-version = "1.97"` for exactly this inheritance. The crate therefore declares no MSRV of its own. Same basis as TASK-2504 (ops-tfplan), TASK-2525 (ops-extension), TASK-2495 (ops-cargo-toml) and the other extension-crate siblings.

**Why it matters**: A library with no declared MSRV has an accidental one that changes with whatever the maintainer happened to compile with (API-15). Version-gated review rules (VER-*) are also gated on the declared MSRV, so the omission leaves the crate's true floor unanswerable from its manifest.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Cargo.toml declares the MSRV via rust-version.workspace = true (or an explicit rust-version matching the workspace floor)
- [x] #2 cargo check -p ops-rust-loc still succeeds and cargo metadata reports the rust-version for ops-rust-loc

<!-- AC:END -->
