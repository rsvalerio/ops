---
id: TASK-2808
title: 'SEC-33: git diff stderr drain reads the child''s stderr with no size cap'
status: Done
assignee: []
created_date: '2026-11-18 18:46'
updated_date: '2026-11-18 22:30'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2861'
modified_files:
  - extensions/hook-common/src/git_state.rs
priority: low
ordinal: 1000
dedup_key: 'SEC-33:extensions/hook-common/src/git_state.rs:spawn_stderr_drain'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/git_state.rs:210` (`spawn_stderr_drain`), consumed at `git_state.rs:253` (`classify_diff_exit`)

**What**: The drain thread runs `stderr_pipe.read_to_end(&mut buf)` with no bound, and the whole buffer is then lossily decoded and embedded in `HasStagedFilesError::NonZeroExit { stderr }`, whose `Display` interpolates it. `program` may be a git wrapper, and the function doc (`has_staged_files_with_timeout`, "Single-shot-process only") already acknowledges the buffer is unbounded.

**Why it matters**: A chatty or misbehaving child (or an orphaned grandchild holding the pipe) can grow the buffer without limit for the life of the process, and an arbitrarily large stderr is rendered verbatim into the hook's error output. Only the tail of git's stderr is ever useful in a diagnostic; reading through `Read::take(MAX_STDERR_BYTES)` and draining the rest to `io::sink()` bounds memory without changing the timeout contract.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The drain thread keeps at most a named MAX_* number of stderr bytes (remaining bytes are discarded, not buffered)
- [x] #2 The NonZeroExit stderr field indicates when output was truncated
- [x] #3 A test drives a child that writes more than the cap to stderr and asserts the captured stderr length is bounded

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.27 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
