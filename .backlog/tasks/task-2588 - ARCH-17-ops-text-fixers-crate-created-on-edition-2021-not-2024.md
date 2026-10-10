---
id: TASK-2588
title: 'ARCH-17: ops-text-fixers crate created on edition 2021, not 2024'
status: To Do
assignee: []
created_date: '2026-10-10 15:46'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - arch
dependencies: []
parent_task_id: 'TASK-2608'
modified_files:
  - extensions/text-fixers/Cargo.toml
priority: medium
ordinal: 1000
dedup_key: 'ARCH-17:extensions/text-fixers/Cargo.toml:ops-text-fixers'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/text-fixers/Cargo.toml:5`

**What**: The crate inherits `edition.workspace = true`, and the workspace package table pins `edition = "2021"` (root Cargo.toml), so this crate was created on an edition one generation behind. This is the same per-crate finding already filed for the other reviewed extension crates (ops-about-go TASK-2475, ops-about-java TASK-2476, ops-about-python TASK-2479, ops-rust-loc TASK-2545, ops-config-checkers TASK-2547, ops-run-before-commit TASK-2554, ops-git TASK-2562, ops-run-before-push TASK-2576), all of which inherit the same workspace key.

**Why it matters**: ARCH-17 — a new crate should be created on the latest stable edition; an old edition on new code locks the crate out of let-chains (FN-2), the 2024 match-ergonomics rules, and the `unsafe_op_in_unsafe_fn` default, and buys nothing in compatibility since editions are per-crate. The durable fix is workspace-wide (bump `[workspace.package] edition` after `cargo fix --edition`), which these per-crate findings roll up into.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The crate compiles on edition 2024, either via a workspace-wide edition bump or a per-crate override, after cargo fix --edition applies the migration
- [ ] #2 cargo test -p ops-text-fixers passes on the new edition
<!-- AC:END -->
