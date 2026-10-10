---
id: TASK-2475
title: 'ARCH-17: ops-about-go was created on edition 2021, not 2024'
status: Done
assignee: []
created_date: '2026-10-10 15:28'
updated_date: '2026-10-10 22:31'
labels:
  - code-review
  - arch
dependencies: []
parent_task_id: 'TASK-2608'
modified_files:
  - extensions-go/about/Cargo.toml
priority: low
ordinal: 1000
dedup_key: 'ARCH-17:extensions-go/about/Cargo.toml:ops-about-go'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/Cargo.toml:5`

**What**: The crate (first committed 2026-04-06, well after edition 2024 stabilized) sets `edition.workspace = true`, inheriting `edition = "2021"` from `[workspace.package]`. Editions are per-crate: the manifest could carry `edition = "2024"` directly.

**Why it matters**: ARCH-17 — an old edition on new code buys nothing and locks the crate out of let-chains (FN-2), the 2024 match-ergonomics rules, and `unsafe_op_in_unsafe_fn` defaults. Note for triage: the whole workspace inherits 2021, so this may be decided as a workspace-level migration rather than a per-crate fix — flagging it here because the skill's EDITION reference explicitly makes new-crate edition choice ARCH-17's finding-generating scope.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Crate builds with edition = "2024" in extensions-go/about/Cargo.toml, or triage records an explicit workspace-level edition decision covering all member crates
- [x] #2 cargo check -p ops-about-go passes after the change

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Edition 2024 applied as a crate-level override (edition = "2024"). No rustc migration lints fired; rustfmt 2024 style-edition reformat applied, and clippy collapsible_if let-chain collapses where the new edition unlocked them (see TASK-2585 note). Crate tests green after.
<!-- SECTION:NOTES:END -->
