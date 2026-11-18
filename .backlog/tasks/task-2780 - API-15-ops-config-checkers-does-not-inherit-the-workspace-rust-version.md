---
id: TASK-2780
title: 'API-15: ops-config-checkers does not inherit the workspace rust-version'
status: To Do
assignee: []
created_date: '2026-11-18 18:44'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2847'
modified_files:
  - extensions/config-checkers/Cargo.toml
priority: low
ordinal: 1000
dedup_key: 'API-15:extensions/config-checkers/Cargo.toml:package'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/Cargo.toml:1`

**What**: The `[package]` table inherits `version` and `license` from `[workspace.package]` but not `rust-version` (root `Cargo.toml:11` declares `rust-version = "1.97"`). The crate therefore declares no MSRV of its own, so cargo's MSRV-aware resolution and `rust-version` checks do not apply to it even though `clippy.toml` assumes msrv 1.97 for it.

The same omission exists in several sibling manifests (crates/core, crates/backlog, crates/runner, extensions/tokei, extensions/about, ...); this task covers config-checkers only.

**Why it matters**: The MSRV contract stated for the workspace is not attached to this crate's manifest, so it can silently drift.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 extensions/config-checkers/Cargo.toml has rust-version.workspace = true
- [ ] #2 cargo metadata reports rust_version 1.97 for ops-config-checkers
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.23 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
