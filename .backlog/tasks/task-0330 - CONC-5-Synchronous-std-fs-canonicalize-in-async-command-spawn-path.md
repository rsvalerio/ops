---
id: TASK-0330
title: 'CONC-5: Synchronous std::fs::canonicalize in async command spawn path'
status: Done
assignee:
  - TASK-0414
created_date: '2026-04-26 09:32'
updated_date: '2026-04-26 10:58'
labels:
  - code-review-rust
  - concurrency
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/build.rs:84-85` (also resolve_spec_cwd line 158)

**What**: `detect_workspace_escape` calls `std::fs::canonicalize` twice synchronously, and `resolve_spec_cwd` canonicalizes again at line 158. These are invoked by `build_command` from the async `exec_command` hot path on every spawn.

**Why it matters**: Each spawn blocks the tokio worker on filesystem syscalls (slow on NFS or large symlink chains). Under high parallel-command counts this starves other tasks scheduled on the same worker and degrades throughput.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Move canonicalize calls off the runtime via tokio::task::spawn_blocking or tokio::fs equivalents, or hoist resolution out of the per-spawn path
- [ ] #2 Add a regression test demonstrating that a slow filesystem (or a tempdir with many symlink components) does not block an unrelated tokio task running concurrently
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Wave 25 (TASK-0414) review: deferred. Moving canonicalize off the runtime requires changing `build_command` from sync to async (or wrapping it in `tokio::task::spawn_blocking`), which propagates through every caller in `command/exec.rs` and `command/build.rs` plus all `cfg(test)` constructions. The accompanying regression test (slow-fs concurrent task) needs a synthetic slow-canonicalize seam. This is wave-scope work on its own — not safe to bundle with the smaller display/result fixes that landed in wave 25. Reschedule into its own wave with the build_command sync→async migration as the primary change.

Wave 25 actually closed: build_command_async wraps build_command in tokio::task::spawn_blocking so canonicalize syscalls run on the blocking pool. exec_command and exec_command_raw call the async variant. AC#2 covered by build_command_async_does_not_starve_concurrent_tokio_task (current_thread runtime; concurrent counter task makes progress through repeated build_command_async invocations).
<!-- SECTION:NOTES:END -->
