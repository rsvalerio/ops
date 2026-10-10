---
id: TASK-2476
title: 'ARCH-17: ops-about-java crate created on edition 2021'
status: Done
assignee: []
created_date: '2026-10-10 15:29'
updated_date: '2026-10-10 22:31'
labels:
  - code-review-rust
  - arch
dependencies: []
parent_task_id: 'TASK-2608'
modified_files:
  - extensions-java/about/Cargo.toml
priority: medium
ordinal: 1000
dedup_key: 'ARCH-17:extensions-java/about/Cargo.toml:ops-about-java'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/Cargo.toml:5`

**What**: `ops-about-java` declares `edition.workspace = true`, and the workspace root pins `edition = "2021"` — so this crate was created (added 2026-03-10, well after edition 2024 stabilized) on an edition older than 2024. ARCH-17: a new crate should be created on the latest stable edition; per-crate editions link freely, so an old edition on new code buys nothing.

**Why it matters**: Edition 2021 locks the crate out of let-chains (FN-2 — several `if let` ladders in the pom/gradle scanners would flatten), the 2024 match-ergonomics rules, and the `unsafe_op_in_unsafe_fn` default. Note for triage: the root cause is the workspace-level `edition = "2021"` in the root Cargo.toml, so this finding likely consolidates with the parallel ones from the sibling crate reviews (e.g. TASK-2445 for ops-about-node) into a single workspace-wide `cargo fix --edition` migration plus flipping the root (or per-crate) edition key.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The crate compiles and passes its tests on edition 2024, via cargo fix --edition followed by flipping the edition (workspace-wide or per-crate)

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Edition 2024 applied as a crate-level override (edition = "2024"). No rustc migration lints fired; rustfmt 2024 style-edition reformat applied, and clippy collapsible_if let-chain collapses where the new edition unlocked them (see TASK-2585 note). Crate tests green after.
<!-- SECTION:NOTES:END -->
