---
id: TASK-0217
title: >-
  TEST-11: run_plan_unknown_command_emits_failure asserts only substring
  presence on message
status: Done
assignee: []
created_date: '2026-04-23 06:32'
updated_date: '2026-04-23 14:59'
labels:
  - rust-code-review
  - test
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/tests.rs:308`

**What**: Test checks message.contains("unknown command") without verifying the id or exact message format.

**Why it matters**: Loose substring checks tolerate real regressions (e.g., wrong id) that still match substring.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Assert equality against the exact expected message or verify id substring too
- [ ] #2 Factor a shared helper for failure-message assertions
<!-- AC:END -->
