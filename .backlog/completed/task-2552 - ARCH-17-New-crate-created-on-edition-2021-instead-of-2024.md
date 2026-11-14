---
id: TASK-2552
title: 'ARCH-17: New crate created on edition 2021 instead of 2024'
status: Done
assignee: []
created_date: '2026-10-10 15:39'
updated_date: '2026-10-10 22:31'
labels:
  - code-review
  - architecture
dependencies: []
parent_task_id: 'TASK-2608'
modified_files:
  - extensions/create-review-tasks/Cargo.toml
priority: low
ordinal: 1000
dedup_key: 'ARCH-17:extensions/create-review-tasks/Cargo.toml:ops-create-review-tasks'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/create-review-tasks/Cargo.toml:5`

**What**: The crate was created on 2026-08-26 (commit 24a3e6b1, "feat(create-review-tasks): add the backlog review-request task engine") with `edition.workspace = true`, inheriting the workspace's `edition = "2021"` — even though edition 2024 was long stable and the workspace's `rust-version` is 1.97, far past 2024's stabilization (1.85). ARCH-17's scope is exactly this: the edition a crate is created on. The rule preempts the workspace-inheritance defense — editions are per-crate, a 2021-edition binary links 2024-edition libraries without issue, so an old edition on new code buys nothing.

**Why it matters**: ARCH-17: a new crate should be created on the latest stable edition. Staying on 2021 locks this crate out of let-chains (FN-2), the `unsafe_op_in_un_safe_fn` default, and the 2024 match-ergonomics rules, for zero compatibility gain. A per-crate `edition = "2024"` override is a one-line change compatible with the rest of the workspace; if the owner prefers a single workspace-wide migration instead, the finding can be declined with that rationale recorded.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The crate builds on edition 2024, either via a per-crate edition = "2024" in its Cargo.toml or by recorded decision to migrate with the workspace
- [x] #2 cargo check -p ops-create-review-tasks and cargo test -p ops-create-review-tasks pass on the new edition

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Edition 2024 applied as a crate-level override (edition = "2024"). No rustc migration lints fired; rustfmt 2024 style-edition reformat applied, and clippy collapsible_if let-chain collapses where the new edition unlocked them (see TASK-2585 note). Crate tests green after.
<!-- SECTION:NOTES:END -->
