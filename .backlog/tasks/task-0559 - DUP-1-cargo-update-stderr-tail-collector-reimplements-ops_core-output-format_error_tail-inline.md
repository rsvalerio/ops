---
id: TASK-0559
title: >-
  DUP-1: cargo-update stderr-tail collector reimplements
  ops_core::output::format_error_tail inline
status: Done
assignee:
  - TASK-0645
created_date: '2026-04-29 05:03'
updated_date: '2026-04-29 17:43'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:279-287`

**What**: The non-zero-exit error path does lines().rev().take(10).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join with newline — three intermediate Vec allocations to format a 10-line tail. Sister extensions (test-coverage/lib.rs:109, metadata/lib.rs:41) already route through ops_core::output::format_error_tail for exactly this.

**Why it matters**: Three concrete copies of the same error-tail format encourage future drift; new authors will keep replicating the inline form because the canonical helper isn`t visible from this site.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace the inline lines/rev/take/collect chain with ops_core::output::format_error_tail(&output.stderr, 10)
- [ ] #2 Confirm error-message shape still matches existing tests
<!-- AC:END -->
