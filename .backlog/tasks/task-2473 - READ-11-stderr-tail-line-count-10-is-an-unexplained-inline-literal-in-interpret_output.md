---
id: TASK-2473
title: 'READ-11: stderr-tail line count 10 is an unexplained inline literal in interpret_output'
status: To Do
assignee: []
created_date: '2026-10-10 15:28'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2615'
modified_files:
  - extensions-rust/cargo-update/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'READ-11:extensions-rust/cargo-update/src/lib.rs:interpret_output'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:649`

**What**: `interpret_output` calls `format_error_tail(&output.stderr, 10)` with a bare inline literal. There is no named `const` and no comment saying why 10 lines is the right tail for a failed `cargo update --dry-run`, what breaks if it changes, or that it is pinned to cargo's error-report shape. Sibling call sites are also inconsistent: `extensions-rust/metadata` and `extensions-rust/test-coverage` use a bare `5` (each equally unexplained), so the workspace has two different tail sizes with no recorded rationale for either.

**Why it matters**: READ-11 targets exactly this shape — a buffer/batch-size literal that is load-bearing folklore. The tail length decides how much of a failed run's diagnostics an operator sees; the next person to touch it has no way to know whether 10 was measured against cargo's typical error output or picked arbitrarily.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The tail line count is a named crate-level const with a doc comment stating why that value and what it is pinned to
- [ ] #2 interpret_output references the const instead of the inline literal
<!-- AC:END -->
