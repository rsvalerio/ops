---
id: TASK-2630
title: 'READ-11: sibling format_error_tail call sites use an unexplained inline literal 5'
status: Triage
assignee: []
created_date: '2026-10-10 21:46'
labels:
  - code-review-rust
  - READ
dependencies: []
modified_files:
  - extensions-rust/metadata/src/lib.rs
  - extensions-rust/test-coverage/src/subprocess.rs
  - extensions-rust/test-coverage/src/parse.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/lib.rs:247` (also `extensions-rust/test-coverage/src/subprocess.rs:134`, `extensions-rust/test-coverage/src/parse.rs:314` and `:428`)

**What**: Four `format_error_tail(&stderr, 5)` call sites pass a bare inline literal, with no named `const` and no comment saying why 5 lines is the right tail, what breaks if it changes, or which tool's error-report shape it is pinned to. ops-cargo-update's counterpart was fixed as `STDERR_TAIL_LINES = 10` with a rationale doc comment (naming the two tail sizes and their grounds); these siblings remain folklore literals.

**Why it matters**: READ-13/READ-11 — the tail length decides how much of a failed subprocess's diagnostics an operator sees. The workspace currently has two different tail sizes (5 here, 10 in cargo-update) with a recorded rationale for neither of the 5s; the next person to touch either has no way to know whether the value was measured or picked arbitrarily.

**Origin**: discovered during TASK-2615 while fixing TASK-2473, whose finding named these sibling call sites but whose acceptance criteria were scoped to ops-cargo-update only.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each format_error_tail tail length is a named crate-level const with a doc comment stating why that value and what it is pinned to
- [ ] #2 All four call sites reference the const instead of the inline literal
<!-- AC:END -->
