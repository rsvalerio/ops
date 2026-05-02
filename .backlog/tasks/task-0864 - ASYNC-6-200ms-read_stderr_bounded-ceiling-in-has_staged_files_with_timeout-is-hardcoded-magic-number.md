---
id: TASK-0864
title: >-
  ASYNC-6: 200ms read_stderr_bounded ceiling in has_staged_files_with_timeout is
  hardcoded magic number
status: Done
assignee: []
created_date: '2026-05-02 09:20'
updated_date: '2026-05-02 10:39'
labels:
  - code-review-rust
  - async
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:229`

**What**: The 200 ms read_stderr_bounded(&stderr_rx, Duration::from_millis(200), program) ceiling is a magic number with no const, no env override, and no rationale beyond the comment "the drain thread should finish immediately". On a slow CI runner with a wedged grandchild, 200 ms can race against legitimate stderr drains for git emitting a multi-line warning before exit.

**Why it matters**: Bounded waits in this crate are otherwise centralised behind DEFAULT_GIT_TIMEOUT / MAX_GIT_TIMEOUT_SECS. This one is not, so an operator tuning the env var still gets a hidden 200ms cap that can clip diagnostic stderr on slow hosts. The output ends up empty rather than partial - silently degrading the error message in HasStagedFilesError::NonZeroExit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Promote 200ms to a named const STDERR_DRAIN_GRACE: Duration with a comment explaining the choice
- [ ] #2 Add a unit test where a fake git emits stderr 100ms before exiting, then exits cleanly with status 128, asserting the stderr is captured in NonZeroExit { stderr, .. }
- [ ] #3 Document the relationship to MAX_GIT_TIMEOUT_SECS so future tuning has a single touch-point
<!-- AC:END -->
