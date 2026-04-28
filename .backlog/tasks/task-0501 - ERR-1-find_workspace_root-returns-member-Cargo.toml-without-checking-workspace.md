---
id: TASK-0501
title: >-
  ERR-1: find_workspace_root returns member Cargo.toml without checking
  [workspace]
status: To Do
assignee:
  - TASK-0533
created_date: '2026-04-28 06:50'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/lib.rs:270`

**What**: find_workspace_root walks up from `start` and returns the first directory containing a Cargo.toml. From inside a member crate, it returns the member manifest, not the workspace root.

**Why it matters**: All Rust providers (identity, units, coverage, deps) call load_workspace_manifest -> CargoTomlProvider, which uses this function. Running `ops about` from `cd crates/foo` resolves to the member manifest, so [workspace] is None and module_count, units list, coverage units silently empty out.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 When start is inside a workspace member, find_workspace_root returns the directory whose Cargo.toml contains [workspace]
- [ ] #2 Test added covering cd crates/foo discovering parent workspace root
<!-- AC:END -->
