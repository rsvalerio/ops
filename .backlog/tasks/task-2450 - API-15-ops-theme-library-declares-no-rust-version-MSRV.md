---
id: TASK-2450
title: 'API-15: ops-theme library declares no rust-version (MSRV)'
status: To Do
assignee: []
created_date: '2026-10-10 15:24'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2608'
modified_files:
  - crates/theme/Cargo.toml
priority: low
ordinal: 1000
dedup_key: 'API-15:crates/theme/Cargo.toml:ops-theme'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/Cargo.toml`

**What**: `cargo metadata --no-deps` reports `rust_version: null` for ops-theme. The workspace root declares `[workspace.package] rust-version = \"1.97\"`, but workspace.package fields are only inherited by members that opt in with `rust-version.workspace = true`, which this manifest does not do (only crates/cli declares it).

**Why it matters**: API-15 — a library with no declared MSRV has an accidental one that changes with whatever compiler the maintainer happened to build with; code in this crate already leans on recent stabilities, so the floor should be stated, not implied.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 crates/theme/Cargo.toml declares rust-version.workspace = true (inheriting the workspace's 1.97) or an explicit rust-version
- [ ] #2 cargo metadata --no-deps reports a non-null rust_version for ops-theme
<!-- AC:END -->
