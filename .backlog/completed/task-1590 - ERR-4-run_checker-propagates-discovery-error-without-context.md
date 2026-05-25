---
id: TASK-1590
title: 'ERR-4: run_checker propagates discovery error without context'
status: Done
assignee:
  - TASK-1636
created_date: '2026-05-21 22:48'
updated_date: '2026-05-22 12:17'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/lib.rs:140`

**What**: `ops_text_fixers::discovery::discover(&opts.root, opts.tracked_only)?` propagates the underlying error with no `.with_context(...)` describing what operation failed or which root/checker label was running. When a `check-json` or `check-yaml` run fails during discovery, the user gets a bare anyhow chain (e.g. an `io::Error` from `git ls-files` or a directory walk) with no indication that it came from the checker, which root path was being walked, or whether the failure was in tracked-only mode.

**Why it matters**: This is the only fallible call before the per-file loop. Without context, diagnosing checker failures (especially in CI where the run is invoked indirectly via the `ops check-json` / `ops check-yaml` registered commands) requires reading source to map the error back to its origin. Adding context here is a one-liner and matches the anyhow conventions used elsewhere in the workspace.

**Scope**: also re-check `run_check_json` / `run_check_yaml` wrappers — they currently add no surrounding context either; the context can be added once at the `?` site in `run_checker` since `label` is already in scope.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 discover()? in run_checker wraps the error via anyhow::Context with the checker label and root path
- [x] #2 Error messages from a failing check-json/check-yaml run identify which checker and which root failed before the underlying cause
<!-- AC:END -->
