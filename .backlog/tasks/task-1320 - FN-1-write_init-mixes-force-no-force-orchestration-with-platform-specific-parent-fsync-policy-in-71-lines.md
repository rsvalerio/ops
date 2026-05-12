---
id: TASK-1320
title: >-
  FN-1: write_init mixes force/no-force orchestration with platform-specific
  parent fsync policy in 71 lines
status: Done
assignee:
  - TASK-1386
created_date: '2026-05-11 20:49'
updated_date: '2026-05-12 23:43'
labels:
  - code-review-rust
  - FN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/init_cmd.rs:73`

**What**: `fn write_init(path: &Path, bytes: &[u8], force: bool)` spans 71 lines (73 to 144) and mixes three concerns:
1. Force/no-force branching (early `if !force`).
2. `OpenOptions::create_new` write + `sync_all` on the no-force path.
3. Two `#[cfg(...)]` branches (unix / not-unix) for the parent-directory fsync, including the open-or-warn / sync-or-warn fallback ladder copied from `edit::atomic_write`.

A single function now performs the role of an orchestrator (which branch to take), a low-level platform-specific durability primitive (parent fsync with two failure modes per Unix branch), and a Windows-portability commentary block. Per FN-1 each function should operate at one abstraction level; the function body reads as three nested abstraction levels because the unix branch alone is ~25 lines of opening / syncing / warning.

**Why it matters**: New durability concerns (e.g. SEC-25 follow-ups, sync_all on the temp file before rename, or an additional platform — illumos, fuchsia) require editing the orchestrator instead of extending a named `parent_fsync_after_create(&Path)` helper. The TASK-1096 / TASK-1231 fixes both landed *inside* this function, demonstrating that future edits gravitate here rather than splitting cleanly. Extracting `parent_fsync_after_create` (or `fsync_new_file_unix` / `fsync_new_file_windows_noop`) would keep `write_init` at the orchestrator layer and isolate the platform branches for unit testing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 write_init reads as orchestration only (force-vs-no-force branching + delegation), no inline parent-fsync ladder
- [x] #2 Parent-directory fsync logic is extracted into a named helper (unix + non-unix), exercised by its own unit test or doc-test
<!-- AC:END -->
