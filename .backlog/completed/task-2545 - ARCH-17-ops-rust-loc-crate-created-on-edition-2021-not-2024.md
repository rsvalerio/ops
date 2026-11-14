---
id: TASK-2545
title: 'ARCH-17: ops-rust-loc crate created on edition 2021, not 2024'
status: Done
assignee: []
created_date: '2026-10-10 15:38'
updated_date: '2026-10-10 22:31'
labels:
  - code-review
  - architecture
dependencies: []
parent_task_id: 'TASK-2608'
modified_files:
  - extensions-rust/loc/Cargo.toml
priority: medium
ordinal: 1000
dedup_key: 'ARCH-17:extensions-rust/loc/Cargo.toml:ops-rust-loc'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/loc/Cargo.toml:5`

**What**: The crate was added 2026-08-09 (commit 8b9a55f0, PR #8) but inherits the workspace's `edition = "2021"` via `edition.workspace = true` (workspace root pins `edition = "2021"`). ARCH-17 requires a newly created crate to use the latest stable edition (2024). Same basis as the ops-about-node (TASK-2445), ops-about-go (TASK-2475), ops-about-java (TASK-2476) and ops-about-python (TASK-2479) sibling findings.

**Why it matters**: Editions are per-crate, so an old edition on new code buys nothing and locks the crate out of let-chains (FN-2), the `unsafe_op_in_unsafe_fn` default, and the 2024 match-ergonomics rules. Every future edit to this crate pays the tax.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Crate compiles on edition 2024 (crate-level edition override or a workspace bump), with cargo fix --edition applied if the compiler requests any migration fixes
- [x] #2 cargo test -p ops-rust-loc passes unchanged

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Edition 2024 applied as a crate-level override (edition = "2024"). No rustc migration lints fired; rustfmt 2024 style-edition reformat applied, and clippy collapsible_if let-chain collapses where the new edition unlocked them (see TASK-2585 note). Crate tests green after.
<!-- SECTION:NOTES:END -->
