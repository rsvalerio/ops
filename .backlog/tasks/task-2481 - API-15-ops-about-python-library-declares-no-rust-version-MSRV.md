---
id: TASK-2481
title: 'API-15: ops-about-python library declares no rust-version (MSRV)'
status: To Do
assignee: []
created_date: '2026-10-10 15:30'
updated_date: '2026-10-10 21:12'
labels:
  - code-review
  - api
dependencies: []
parent_task_id: 'TASK-2608'
modified_files:
  - extensions-python/about/Cargo.toml
priority: low
ordinal: 1000
dedup_key: 'API-15:extensions-python/about/Cargo.toml:ops-about-python'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/Cargo.toml:1`

**What**: The crate is a lib target but declares no `rust-version` in its Cargo.toml (neither a local value nor `rust-version.workspace = true`), so its MSRV is accidental — whatever compiler the last maintainer happened to build with. Same basis as the ops-theme sibling finding (TASK-2450).

**Why it matters**: Rules gated on stabilization versions (VER-*, let-chains in FN-2) are review-checkable only against a declared MSRV; without one, a dependency or toolchain bump silently raises it. `publish = false` limits the blast radius to this workspace, which pins `rust-version = "1.97"` at the root — inheriting it explicitly makes the crate's contract match the workspace's.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Cargo.toml declares the crate's MSRV (rust-version = workspace value, via rust-version.workspace = true or an explicit pin matching the root)
- [ ] #2 cargo check -p ops-about-python succeeds with the declared toolchain
<!-- AC:END -->
