---
id: TASK-2479
title: 'ARCH-17: ops-about-python crate created on edition 2021'
status: To Do
assignee: []
created_date: '2026-10-10 15:29'
updated_date: '2026-10-10 21:12'
labels:
  - code-review
  - architecture
dependencies: []
parent_task_id: 'TASK-2608'
modified_files:
  - extensions-python/about/Cargo.toml
priority: medium
ordinal: 1000
dedup_key: 'ARCH-17:extensions-python/about/Cargo.toml:ops-about-python'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/Cargo.toml:5`

**What**: The crate was added 2026-04-19 (commit 1510e44c) but inherits the workspace's `edition = "2021"` via `edition.workspace = true` (workspace root pins `edition = "2021"`). ARCH-17 requires a newly created crate to use the latest stable edition (2024). Same basis as the ops-about-node sibling finding (TASK-2445).

**Why it matters**: Editions are per-crate, so an old edition on new code buys nothing and locks the crate out of let-chains (FN-2), the `unsafe_op_in_unsafe_fn` default, and the 2024 match-ergonomics rules. Every future edit to this crate pays the tax.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Crate compiles on edition 2024 (crate-level edition override or a workspace bump), with cargo fix --edition applied if the compiler requests any migration fixes
- [ ] #2 cargo test -p ops-about-python passes unchanged
<!-- AC:END -->
