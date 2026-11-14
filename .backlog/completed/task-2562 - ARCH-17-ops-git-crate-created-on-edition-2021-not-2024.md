---
id: TASK-2562
title: 'ARCH-17: ops-git crate created on edition 2021, not 2024'
status: Done
assignee: []
created_date: '2026-10-10 15:40'
updated_date: '2026-10-10 22:31'
labels:
  - code-review-rust
  - architecture
dependencies: []
parent_task_id: 'TASK-2608'
modified_files:
  - extensions/git/Cargo.toml
priority: low
ordinal: 1000
dedup_key: 'ARCH-17:extensions/git/Cargo.toml:[package]'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/Cargo.toml:5`

**What**: The crate inherits `edition.workspace = true`, which resolves to the workspace-wide `edition = "2021"`. extensions/git was created 2026-04-18 (first commit 043e6b5c era tree, `git log --diff-filter=A -- extensions/git/*`), long after edition 2024 stabilized — so the crate was created on an old edition rather than migrated to one.

**Why it matters**: ARCH-17: a new crate should be created on the latest stable edition; it costs nothing in compatibility (editions are per-crate; a 2021 binary links 2024 libraries) and an old edition on new code locks the crate out of let-chains (FN-2), `gen`, the `unsafe_op_in_unsafe_fn` default, and the 2024 match-ergonomics rules. This crate's guard-heavy parsing (`split_owner_repo`, `parse_origin_url_inner`) is exactly the let-chain shape 2024 enables. Sibling extension crates created on the workspace 2021 edition carry the same finding (TASK-2445, TASK-2475, TASK-2545, TASK-2554).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 extensions/git/Cargo.toml declares edition 2024 (overriding the workspace-inherited 2021) after running cargo fix --edition on the crate
- [x] #2 cargo test -p ops-git passes on the new edition with no behavioral change

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Edition 2024 applied as a crate-level override (edition = "2024"). No rustc migration lints fired; rustfmt 2024 style-edition reformat applied, and clippy collapsible_if let-chain collapses where the new edition unlocked them (see TASK-2585 note). Crate tests green after.
<!-- SECTION:NOTES:END -->
