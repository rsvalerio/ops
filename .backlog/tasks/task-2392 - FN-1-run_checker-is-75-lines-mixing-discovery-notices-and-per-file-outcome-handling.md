---
id: TASK-2392
title: 'FN-1: run_checker is ~75 lines mixing discovery, notices, and per-file outcome handling'
status: Done
assignee: []
created_date: '2026-10-04 14:15'
updated_date: '2026-10-04 15:59'
labels:
  - code-review-rust
  - functions
dependencies: []
parent_task_id: 'TASK-2422'
modified_files:
  - extensions/config-checkers/src/runner.rs
priority: low
ordinal: 1000
dedup_key: 'FN-1:extensions/config-checkers/src/runner.rs:run_checker'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/runner.rs:63-150` (`run_checker`)

**What**: One function does discovery with context, fallback notice, walk-error reporting, the extension filter, the bounded read, and a three-arm outcome match that builds `FailedFile` values and writes skip notices. It runs ~75 lines (limit 50) across several abstraction levels, with the match nested inside a `for` inside the function.

**Why it matters**: Harder to read and test than named steps. Extract e.g. `discover_candidates(opts, writer, label, &mut report)` and `check_one(path, ...)` (returns the outcome, so the skip/failure/parse arms are testable on their own).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 run_checker is <= 50 lines, with discovery and per-file handling extracted into named helpers
- [x] #2 Existing tests in tests.rs pass unchanged

<!-- AC:END -->
