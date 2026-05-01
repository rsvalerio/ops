---
id: TASK-0219
title: >-
  UNSAFE-8: remove_var in run-before-push test mutates process-global env
  without guard
status: Done
assignee: []
created_date: '2026-04-23 06:33'
updated_date: '2026-04-23 07:38'
labels:
  - rust-code-review
  - unsafe
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-push/src/lib.rs:96`

**What**: Same unsafe env mutation pattern as run-before-commit — not covered by existing UNSAFE-8 tasks for hook-common/text_util.

**Why it matters**: Edition 2024 removes implicit unsafety; parallel test flakiness.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Replace with EnvGuard helper
- [x] #2 Serialize the test
<!-- AC:END -->
