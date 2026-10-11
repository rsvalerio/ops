---
id: TASK-2555
title: 'API-15: ops-git declares no rust-version while the workspace package table carries MSRV 1.97'
status: Done
assignee: []
created_date: '2026-10-10 15:40'
updated_date: '2026-10-10 22:26'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2608'
modified_files:
  - extensions/git/Cargo.toml
priority: low
ordinal: 1000
dedup_key: 'API-15:extensions/git/Cargo.toml:[package]'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/Cargo.toml:4-6`

**What**: The `[package]` table inherits `version`, `edition` and `license` from `[workspace.package]` but omits `rust-version.workspace = true`. Cargo does not propagate workspace-package fields without explicit inheritance, so this crate has no declared MSRV even though the workspace pins `rust-version = "1.97"` at the root. Only `crates/cli` among the members inherits it today.

**Why it matters**: API-15: a library with no declared MSRV has an accidental one that changes with whatever toolchain the maintainer happened to compile with, and version-gated review rules (VER-*, let-chains in FN-2) key off the declared MSRV. Blast radius is small here (`publish = false`, internal workspace), so severity is low — the fix is one line and makes the crate's toolchain floor explicit and consistent with the root.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 extensions/git/Cargo.toml sets rust-version.workspace = true so the crate inherits the workspace MSRV (1.97)
- [x] #2 cargo metadata for ops-git reports the resolved rust-version; cargo check -p ops-git unchanged

<!-- AC:END -->
