---
id: TASK-0193
title: >-
  TEST-18: get_terminal_width_default test races with parallel tests reading
  COLUMNS
status: Done
assignee: []
created_date: '2026-04-22 21:26'
updated_date: '2026-04-23 15:12'
labels:
  - rust-code-review
  - TEST
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/text_util.rs:208-216`

**What**: The test removes COLUMNS, calls get_terminal_width, then restores. Other parallel tests reading COLUMNS can observe either the cleared value or the restored value depending on timing.

**Why it matters**: TEST-18 — isolated state per test. Independent of UNSAFE-8 (separate finding), the flakiness vector is real. Mitigation: use serial_test or refactor get_terminal_width to accept a width source so the test does not touch env.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 get_terminal_width is parameterized or the test runs under a serial guard
- [ ] #2 No test in this module mutates process-global environment variables
<!-- AC:END -->
