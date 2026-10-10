---
id: TASK-2525
title: 'API-15: ops-extension declares no rust-version despite the workspace carrying one'
status: To Do
assignee: []
created_date: '2026-10-10 15:33'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2608'
modified_files:
  - crates/extension/Cargo.toml
priority: low
ordinal: 1000
dedup_key: 'API-15:crates/extension/Cargo.toml:ops-extension'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/Cargo.toml:1`

**What**: The crate's manifest inherits `version`, `edition` and `license` from `[workspace.package]` but omits `rust-version`, so the library ships with no declared MSRV. The workspace table does carry one (`rust-version = "1.97"` in the root `[workspace.package]`), and `crates/cli/Cargo.toml:10` already demonstrates the intended inheritance (`rust-version.workspace = true`) — ops-extension and the other library crates simply never picked it up.

**Why it matters**: API-15 — a library with no declared MSRV has an accidental one that changes with whatever the maintainer happened to compile with. The crate uses recent stabilizations (e.g. `Duration::from_mins` in `crates/extension/src/deadline.rs:39`), so the floor is real, not theoretical; declaring it is one line and makes version-gated rules (VER-*, API-15 gating) checkable against a stated number instead of a guess.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 crates/extension/Cargo.toml contains rust-version.workspace = true and cargo metadata reports the crate's rust_version as 1.97
<!-- AC:END -->
