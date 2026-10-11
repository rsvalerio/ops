---
id: TASK-2585
title: 'ARCH-17: ops-tokei crate created on edition 2021, not 2024'
status: Done
assignee: []
created_date: '2026-10-10 15:46'
updated_date: '2026-10-10 22:31'
labels:
  - code-review-rust
  - architecture
dependencies: []
parent_task_id: 'TASK-2608'
modified_files:
  - extensions/tokei/Cargo.toml
priority: medium
ordinal: 1000
dedup_key: 'ARCH-17:extensions/tokei/Cargo.toml:ops-tokei'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/Cargo.toml:5`

**What**: The crate manifest sets `edition.workspace = true`, which resolves to `edition = "2021"` from the root `[workspace.package]` table, for a crate first added 2026-09-12. ARCH-17: a **new** crate should be created on the latest stable edition (2024 at the time of writing). Editions are per-crate — the member can set `edition = "2024"` directly regardless of the workspace-wide default, and a 2021-edition binary links 2024-edition libraries without issue, so staying on 2021 buys nothing in compatibility.

**Why it matters**: Edition 2021 locks the crate out of let-chains (FN-2), the `gen` reserved keyword, the `unsafe_op_in_unsafe_fn` default, and the 2024 match-ergonomics rules. The workspace-level migration of older crates is `cargo fix --edition`'s job and is not a finding — but this crate is new code that never needed the old edition. Sibling crates filed the same finding in this wave (e.g. TASK-2545 for ops-rust-loc).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Crate builds on edition 2024: manifest sets edition = "2024" (directly or via a workspace bump), cargo check + cargo test pass
- [x] #2 No code changes are needed beyond the edition bump (no 2024 migration lints fire); if any fire, they are applied per EDITION-5

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Edition 2024 applied as a crate-level override (edition = "2024"). No rustc migration lints fired; follow-ups applied: rustfmt 2024 style-edition reformat (use-list reordering, tests.rs) and clippy collapsible_if let-chain collapses in ops-git, ops-text-fixers, ops-rust-loc, ops-about-go, ops-about-java, ops-about-node. All crate tests re-run green; workspace ops verify 8/8.
<!-- SECTION:NOTES:END -->
