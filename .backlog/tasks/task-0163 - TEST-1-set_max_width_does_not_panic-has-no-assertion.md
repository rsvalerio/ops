---
id: TASK-0163
title: 'TEST-1: set_max_width_does_not_panic has no assertion'
status: To Do
assignee: []
created_date: '2026-04-22 21:23'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - TEST
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: crates/core/src/table.rs:128-135

**What**: fn set_max_width_does_not_panic calls table.set_max_width(0, 20) and table.set_max_width(99, 20) with no follow-up assert_*, no #[should_panic], no Result return, and no helper-assertion call. The "assertion" is the absence of a panic, but the method already guards with if let Some(col) = self.inner.column_mut(column). The test would pass even if set_max_width were replaced with a no-op body.

**Why it matters**: Mutation-survivor: the test is coverage theater. Either assert the column constraint was applied (e.g., render and observe the width) or delete it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace with a test that inspects the resulting column constraint, or delete
<!-- AC:END -->
