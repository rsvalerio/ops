---
id: TASK-2779
title: 'ARCH-11: config-checkers sets missing_docs as a crate-root attribute instead of in [workspace.lints]'
status: To Do
assignee: []
created_date: '2026-11-18 18:44'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - architecture
dependencies: []
parent_task_id: 'TASK-2847'
modified_files:
  - extensions/config-checkers/src/lib.rs
  - Cargo.toml
priority: low
ordinal: 1000
dedup_key: 'ARCH-11:extensions/config-checkers/src/lib.rs:crate-root-lints'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/lib.rs:28`

**What**: `#![warn(missing_docs)]` is declared at the crate root of this one crate; no other workspace member does this, and `[workspace.lints.rust]` in the root `Cargo.toml` has no `missing_docs` entry. It is also `warn` in a workspace whose policy is `deny` for clippy groups, so it does not gate anything unless CI passes `-D warnings`.

**Why it matters**: Per-crate lint configuration diverges from the single workspace policy in root `Cargo.toml [workspace.lints]`. Either the lint matters workspace-wide and belongs there, or this crate silently enforces (or fails to enforce) something nobody else does.

<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 missing_docs is either declared once in [workspace.lints.rust] (at a level that gates) or removed from the crate root, with no crate-root lint-level attribute left in config-checkers apart from the documented cfg_attr(test) block
- [ ] #2 ops verify / clippy gates pass afterwards
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.23 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
