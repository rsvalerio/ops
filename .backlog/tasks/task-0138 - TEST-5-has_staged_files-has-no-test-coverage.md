---
id: TASK-0138
title: 'TEST-5: has_staged_files has no test coverage'
status: Done
assignee: []
created_date: '2026-04-22 21:16'
updated_date: '2026-04-23 07:36'
labels:
  - rust-code-review
  - test
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:56-63`

**What**: Public function `has_staged_files()` invokes git, parses status output, and maps errors — but has zero unit/integration tests.

**Why it matters**: This guards pre-commit hook behavior for every user; a regression (e.g., mis-parsing `git diff --cached` output, error propagation) would silently break hooks.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Add tests covering: staged files present → Ok(true); no staged files → Ok(false)
- [x] #2 Add a test for the error path (e.g., running outside a git repo) asserting the error is propagated with context
<!-- AC:END -->
