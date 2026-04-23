---
id: TASK-0143
title: >-
  ERR-1: has_staged_files ignores git exit status — treats failed git diff as
  'no staged files'
status: Done
assignee: []
created_date: '2026-04-22 21:21'
updated_date: '2026-04-23 07:36'
labels:
  - rust-code-review
  - ERR
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:56`

**What**: `has_staged_files` runs `git diff --cached --name-only --diff-filter=ACMR` and returns `Ok(!output.stdout.is_empty())`. It never checks `output.status.success()` nor surfaces `output.stderr`. If `git` fails (e.g. not a git repo, binary missing, corrupted index) with non-zero status and empty stdout, the function silently returns `Ok(false)` — the pre-commit hook will then short-circuit as "nothing to check" and allow the commit through.

**Why it matters**: Fail-open behavior on a pre-commit hook path. A transient git failure (index lock, permissions) silently bypasses the checks the hook is meant to enforce. Should return `Err` on non-zero exit, including stderr context.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Check output.status.success() and return Err with stderr context on failure
- [x] #2 Add tests covering the failure path (non-git dir / non-zero git exit)
<!-- AC:END -->
