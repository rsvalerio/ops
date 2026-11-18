---
id: TASK-2815
title: 'TEST-29: Use assert_matches! instead of assert!(matches!(..)) in run-before-commit tests'
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
dedup_key: 'TEST-29:extensions/run-before-commit/src/lib.rs:tests'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:581` (`has_staged_files_errors_when_git_binary_missing`), `:604-607` (`has_staged_files_times_out_on_hanging_git`), `:682` (`has_staged_files_handles_large_output_without_deadlock`)

**What**: Pattern assertions are hand-rolled as `assert!(matches!(err, HasStagedFilesError::Spawn { .. }))` and similar. The workspace MSRV is 1.97 (`Cargo.toml` `rust-version`), so std `assert_matches!` (stable since 1.96) is available.

**Why it matters**: `assert!(matches!(..))` on failure prints only "assertion failed" (`:581` has no message at all), while `assert_matches!` prints the `Debug` of the actual value — the information needed to triage a CI failure in these timing-sensitive probe tests.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The three assert!(matches!(..)) sites use std assert_matches! (imported via use std::assert_matches;)

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.28 (code-review-rust run, 2026-11-18).

The three sites moved to extensions/hook-common/tests/git_state.rs with TASK-2804; they use std assert_matches! there.

<!-- SECTION:NOTES:END -->
