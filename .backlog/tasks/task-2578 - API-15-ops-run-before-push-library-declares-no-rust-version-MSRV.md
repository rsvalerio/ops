---
id: TASK-2578
title: 'API-15: ops-run-before-push library declares no rust-version (MSRV)'
status: To Do
assignee: []
created_date: '2026-10-10 15:44'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2608'
modified_files:
  - extensions/run-before-push/Cargo.toml
priority: low
ordinal: 1000
dedup_key: 'API-15:extensions/run-before-push/Cargo.toml:ops-run-before-push'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-push/Cargo.toml:1`

**What**: The `[package]` table sets `version.workspace = true` but not `rust-version` (nor `rust-version.workspace = true`). The workspace root declares `rust-version = "1.97"` under `[workspace.package]`, but Cargo only applies that to members that explicitly inherit it, so this package has no declared MSRV at all.

**Why it matters**: Per API-15, a library with no declared MSRV has an accidental one that changes with whatever the maintainer happened to compile with. Version-gated rules (VER-*, TEST-29) are gated on the declared MSRV; without it there is no anchor. One-line fix: add `rust-version.workspace = true`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Manifest declares rust-version, e.g. rust-version.workspace = true
- [ ] #2 cargo metadata reports the MSRV for ops-run-before-push
<!-- AC:END -->
