---
id: TASK-2401
title: 'SEC-25: text-fixers write-back re-resolves the path and ignores concurrent edits since the read'
status: To Do
assignee: []
created_date: '2026-10-04 14:16'
updated_date: '2026-10-04 14:52'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2422'
modified_files:
  - extensions/text-fixers/src/runner.rs
  - extensions/text-fixers/src/atomic.rs
priority: low
ordinal: 1000
dedup_key: 'SEC-25:extensions/text-fixers/src/runner.rs:run_fixer'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/text-fixers/src/runner.rs:run_fixer` (call to `atomic::replace`), `extensions/text-fixers/src/atomic.rs:replace`

**What**: The read half is handle-based and symlink-refusing (`read_candidate` -> `open_refusing_symlinks`, fstat on the descriptor). The write half is not: `atomic::replace(&path, ...)` re-resolves `path` by name, creating the stage file in `path.parent()` and `persist`ing over `path`. Between read and rename (a) a directory component can be swapped for a symlink, so the stage file and rename land outside `root`; (b) the file itself can be edited by the user/editor/another step, and the rename silently discards that edit because nothing compares the on-disk identity/mtime/len against the `Metadata` taken at read time (it is only used for mode/uid/gid).

**Why it matters**: The tool runs from a pre-commit hook over the whole worktree and rewrites in place; the same check-then-use gap the read side was hardened against (SEC-25) reopens on write, and a lost concurrent edit is silent data loss.

Scanning note: `replace` documents that `original` must come from the read handle, but never uses it for a changed-since-read check.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Before persist, replace re-stats the target (no-follow) and refuses/skips with a reported failure when dev/ino/mtime/len differ from the read-time Metadata
- [ ] #2 Write-back refuses a symlinked parent component (or is performed relative to a directory handle) with a test for a swapped parent
- [ ] #3 Tests cover a file modified between read and replace being left untouched and reported
<!-- AC:END -->
