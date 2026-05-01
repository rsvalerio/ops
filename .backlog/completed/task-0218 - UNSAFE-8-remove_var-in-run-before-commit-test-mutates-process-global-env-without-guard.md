---
id: TASK-0218
title: >-
  UNSAFE-8: remove_var in run-before-commit test mutates process-global env
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
**File**: `extensions/run-before-commit/src/lib.rs:110`

**What**: Uses std::env::remove_var in #[test] fn without EnvGuard; remove_var is unsafe in 2024 edition and races with parallel tests.

**Why it matters**: Concurrent tests reading/writing SKIP_OPS_RUN_BEFORE_COMMIT can flake; 2024 edition makes this a hard error without unsafe block.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Wrap env mutation in an EnvGuard serial helper
- [x] #2 Mark test with #[serial] or remove the mutation
<!-- AC:END -->
