---
id: TASK-1336
title: 'TEST-11: run_command_returns_error_for_unknown_command asserts only is_err()'
status: To Do
assignee:
  - TASK-1385
created_date: '2026-05-12 16:27'
updated_date: '2026-05-12 22:16'
labels:
  - code-review-rust
  - tests
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd/tests.rs:111-129` (and sibling `run_command_returns_error_for_cycle` at 181-199)

**What**: Both tests name a specific failure mode (unknown command / cycle) but only assert `result.is_err()`. A regression where the function fails for an unrelated reason (config-load error after a refactor, etc.) still passes. The codebase has the inverse pattern in `dry_run_returns_error_for_unknown_command:463`, which inspects the error message — that is the standard to match.

**Why it matters**: Vacuous assertions hide regressions in error provenance. The CLI's integration tests already encode the right substring match; the unit tests should match.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 run_command_returns_error_for_unknown_command asserts the error chain contains the missing command name (e.g. "nonexistent").
- [ ] #2 Same fix applied to run_command_returns_error_for_cycle (assert cycle-related substring).
<!-- AC:END -->
