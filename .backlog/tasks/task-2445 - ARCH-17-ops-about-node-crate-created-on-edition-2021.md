---
id: TASK-2445
title: 'ARCH-17: ops-about-node crate created on edition 2021'
status: Done
assignee: []
created_date: '2026-10-10 15:23'
updated_date: '2026-10-10 22:31'
labels:
  - code-review
  - architecture
dependencies: []
parent_task_id: 'TASK-2608'
modified_files:
  - extensions-node/about/Cargo.toml
priority: medium
ordinal: 1000
dedup_key: 'ARCH-17:extensions-node/about/Cargo.toml:ops-about-node'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/Cargo.toml:5`

**What**: The crate was added 2026-04-19 (commit 78705b1e) but inherits the workspace's `edition = "2021"` via `edition.workspace = true` (workspace root pins `edition = "2021"`). ARCH-17 requires a newly created crate to use the latest stable edition (2024).

**Why it matters**: Editions are per-crate, so an old edition on new code buys nothing and locks the crate out of let-chains (FN-2), `gen`, the `unsafe_op_in_unsafe_fn` default, and the 2024 match-ergonomics rules. Every future edit to this crate pays the tax. The crate already uses `let ... else` and would benefit from let-chains in the nested `if let` sites.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Crate compiles on edition 2024 (crate-level edition override or a workspace bump), with cargo fix --edition applied if the compiler requests any migration fixes
- [x] #2 cargo test -p ops-about-node passes unchanged

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Edition 2024 applied as a crate-level override (edition = "2024"). No rustc migration lints fired; rustfmt 2024 style-edition reformat applied, and clippy collapsible_if let-chain collapses where the new edition unlocked them (see TASK-2585 note). Crate tests green after.
<!-- SECTION:NOTES:END -->
