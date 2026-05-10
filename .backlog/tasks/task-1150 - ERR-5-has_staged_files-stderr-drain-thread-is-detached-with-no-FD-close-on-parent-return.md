---
id: TASK-1150
title: >-
  ERR-5: has_staged_files stderr drain thread is detached with no FD-close on
  parent return
status: Done
assignee:
  - TASK-1268
created_date: '2026-05-08 07:43'
updated_date: '2026-05-09 17:32'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:202-208`

**What**: The thread spawned to drain child.stderr is fire-and-forget. After has_staged_files_with_timeout returns, the thread continues blocking on read_to_end if a misbehaving wrapper kept the pipe open via an orphan grandchild. The recv_timeout(STDERR_DRAIN_GRACE) gives the parent a bounded wait, but the thread, its accumulating Vec<u8>, and its FD live for the lifetime of the pipe holder.

**Why it matters**: Pre-commit is short-lived today so impact is zero. The moment has_staged_files runs from an LSP-style host or `ops watch` mode, every hung subprocess pins one drain thread, one pipe FD, and one unbounded buffer for the host's lifetime.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 On wait_timeout return, explicitly drop child.stderr (or close the read FD) so read_to_end on the drain side returns EOF
- [x] #2 Or gate function docs to single-shot-process only with a clear warning forcing a future daemon caller to revisit
<!-- AC:END -->
