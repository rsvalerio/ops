---
id: TASK-2714
title: 'READ-10: lint suppressions use #[allow] without reason; test cast allows are unfulfilled'
status: To Do
assignee: []
created_date: '2026-11-18 18:32'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2846'
modified_files:
  - extensions-go/about/src/modules.rs
  - extensions-go/about/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'review-2667.07-5'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/modules.rs:43`, `extensions-go/about/src/lib.rs:12`

**What**:
- modules.rs:43 — `#[allow(clippy::option_if_let_else)]` with the justification in a preceding `//` comment rather than `#[expect(..., reason = "...")]`.
- lib.rs:12-20 — `#![cfg_attr(test, allow(clippy::unwrap_used, clippy::cast_possible_truncation, clippy::cast_precision_loss, clippy::cast_sign_loss))]`: the crate's tests contain no `as` numeric casts, so the three cast allows suppress nothing. With `#[expect]` they would trip `unfulfilled_lint_expectations`.

**Why it matters**: `#[allow]` outlives the violation it was written for; the cast allows are already stale and silently widen what test code may do under the workspace's deny-level pedantic policy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 modules.rs:43 uses #[expect(clippy::option_if_let_else, reason = "...")] with the existing justification moved into reason
- [ ] #2 The unused cast_* allows are removed from the crate-root cfg_attr(test) block; ops verify/clippy stays clean
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.07 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
