---
id: TASK-2576
title: 'ARCH-17: ops-run-before-push crate created on edition 2021'
status: Done
assignee: []
created_date: '2026-10-10 15:43'
updated_date: '2026-10-10 22:31'
labels:
  - code-review-rust
  - architecture
dependencies: []
parent_task_id: 'TASK-2608'
modified_files:
  - extensions/run-before-push/Cargo.toml
priority: medium
ordinal: 1000
dedup_key: 'ARCH-17:extensions/run-before-push/Cargo.toml:ops-run-before-push'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-push/Cargo.toml:5`

**What**: The crate was added 2026-03-10 (commit 23cef82c, well after edition 2024 stabilized in Feb 2025) but inherits the workspace's `edition = "2021"` via `edition.workspace = true` (workspace root pins `edition = "2021"`). ARCH-17 requires a newly created crate to use the latest stable edition (2024).

**Why it matters**: Editions are per-crate, so an old edition on new code buys nothing and locks the crate out of let-chains (FN-2), `gen`, the `unsafe_op_in_unsafe_fn` default, and the 2024 match-ergonomics rules. Every future edit to this crate pays the tax. Note the root cause is the `[workspace.package]` setting, so the fix may land either in this manifest (`edition = "2024"`) or workspace-wide after `cargo fix --edition`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Crate compiles under edition 2024, either via edition = "2024" in this manifest or a workspace-wide bump with cargo fix --edition applied first
- [x] #2 cargo test -p ops-run-before-push passes on the new edition

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Edition 2024 applied as a crate-level override (edition = "2024"). No rustc migration lints fired; rustfmt 2024 style-edition reformat applied, and clippy collapsible_if let-chain collapses where the new edition unlocked them (see TASK-2585 note). Crate tests green after.
<!-- SECTION:NOTES:END -->
