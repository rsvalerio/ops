---
id: TASK-2813
title: 'TEST-11: has_staged_files_errors_outside_git_repo accepts any error containing "failed"'
status: Done
assignee: []
created_date: '2026-11-18 18:46'
updated_date: '2026-11-18 22:30'
labels:
  - code-review-rust
  - TEST
dependencies: []
parent_task_id: 'TASK-2861'
modified_files:
  - extensions/run-before-commit/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-11:extensions/run-before-commit/src/lib.rs:tests::has_staged_files_errors_outside_git_repo'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:546-554` (`has_staged_files_errors_outside_git_repo`)

**What**: The assertion is `msg.contains("not a git repository") || msg.contains("failed")`. "failed" appears in the `Spawn` error ("failed to run") and most other probe errors, so the test passes for a missing git binary, a spawn failure or a timeout just as well as for the non-repository case it is named after. The variant is never checked.

**Why it matters**: A regression that, for example, turned the non-repo case into a `Spawn` or `Timeout` error (or misclassified exit 128) would still pass. Assert the specific variant (`HasStagedFilesError::NonZeroExit { exit_code: Some(128), .. }` or whichever the probe returns) and the "not a git repository" stderr, as the sibling tests do for their cases.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The test asserts the specific HasStagedFilesError variant returned for a non-repository directory
- [x] #2 The disjunction on the generic substring "failed" is removed

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.28 (code-review-rust run, 2026-11-18).

git outside a repository exits 129 (unknown option cached, --no-index fallback) on current git and 128 on older releases; the test asserts NonZeroExit with Some(128|129) and git's diagnostic. Test moved to extensions/hook-common/tests/git_state.rs with TASK-2804.

<!-- SECTION:NOTES:END -->
