---
id: TASK-0228
title: 'ERR-4: run_about_coverage swallows warm-up provider errors with let _ ='
status: To Do
assignee: []
created_date: '2026-04-23 06:33'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/coverage.rs:52`

**What**: Three `let _ = ctx.get_or_provide(...)` calls discard duckdb/coverage/cargo_toml errors with no log.

**Why it matters**: Debugging "no coverage data available" becomes guesswork.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Log at debug/warn on Err
- [ ] #2 Match errors and only ignore NotFound
<!-- AC:END -->
