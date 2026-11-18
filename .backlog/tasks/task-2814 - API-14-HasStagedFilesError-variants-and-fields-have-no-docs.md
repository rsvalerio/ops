---
id: TASK-2814
title: 'API-14: HasStagedFilesError variants and fields have no docs'
status: To Do
assignee: []
created_date: '2026-11-18 18:46'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - API
dependencies: []
parent_task_id: 'TASK-2861'
modified_files:
  - extensions/hook-common/src/git_state.rs
priority: low
ordinal: 1000
dedup_key: 'API-14:extensions/hook-common/src/git_state.rs:HasStagedFilesError'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/git_state.rs:18-35` (`HasStagedFilesError`)

**What**: `HasStagedFilesError` is the public, `#[non_exhaustive]` typed error of `has_staged_files_with_timeout`, and downstream crates match on its variants (`extensions/run-before-commit/src/lib.rs:447`, `:643`, `:702`). None of its four variants (`Spawn`, `Timeout`, `NonZeroExit`, `Io`) nor their public fields (`program`, `timeout`, `exit_code`, `stderr`, `source`) carries a doc comment. Notably undocumented: that `exit_code` is `None` when the child was killed by a signal, that `stderr` is trimmed and lossily decoded, and that `Io` covers a failed wait rather than a failed read of output despite its message.

**Why it matters**: Callers branch on these variants to decide whether the hook fails or skips, so the conditions under which each one is returned are part of the contract (ERR-2) and belong in rustdoc rather than in the reader's inference from `git_state.rs` internals.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each HasStagedFilesError variant has a doc comment stating when has_staged_files_with_timeout returns it
- [ ] #2 Each public field is documented, including exit_code being None on signal termination and stderr being trimmed lossy UTF-8
- [ ] #3 The Io variant's message or doc accurately describes the failing operation (waiting on the child)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.27 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
