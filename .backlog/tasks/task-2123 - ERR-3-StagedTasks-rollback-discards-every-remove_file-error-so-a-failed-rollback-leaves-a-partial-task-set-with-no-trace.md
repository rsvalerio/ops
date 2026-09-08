---
id: TASK-2123
title: >-
  ERR-3: StagedTasks rollback discards every remove_file error, so a failed
  rollback leaves a partial task set with no trace
status: Done
assignee:
  - TASK-2249
created_date: '2026-09-08 06:54'
updated_date: '2026-09-08 15:42'
labels:
  - code-review-rust
  - error-handling
dependencies: []
modified_files:
  - extensions/create-review-tasks/src/lib.rs
priority: low
ordinal: 39000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/create-review-tasks/src/lib.rs:453` (`impl Drop for StagedTasks`)

**What**: The rollback loop is `let _ = std::fs::remove_file(path);`. The comment justifies best-effort behaviour — correctly, since `Drop` cannot propagate and the caller is already returning an error or retrying — but it discards the error entirely rather than logging it.

**Why it matters**: The whole point of `StagedTasks` is stated in its own doc comment: "a half-created set is worse than none, because the backlog CLI would show a review request whose subtasks silently stop short of the targets it names". If a `remove_file` fails (permissions, a read-only mount, an editor holding the file on Windows), that exact bad state is what remains on disk — and the operator gets only the original error, with nothing indicating the backlog tree was left dirty. The crate already depends on `tracing` and uses `tracing::debug!` on the neighbouring conflict path, so a `tracing::warn!` naming the path and the io error costs nothing and turns a silent bad state into a diagnosable one.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A failed rollback delete emits a tracing warning naming the path that could not be removed and the underlying io error
- [x] #2 The rollback still continues past a failure and still never panics or masks the caller's error
<!-- AC:END -->
