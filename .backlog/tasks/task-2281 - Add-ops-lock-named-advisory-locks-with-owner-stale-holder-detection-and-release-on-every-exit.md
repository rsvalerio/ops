---
id: TASK-2281
title: 'Add ops lock: named advisory locks with owner, stale-holder detection and release on every exit'
status: Done
assignee: []
created_date: '2026-09-26 17:47'
updated_date: '2026-09-26 19:05'
labels:
  - feature
  - cli
  - skills-integration
  - concurrency
dependencies: []
parent_task_id: 'TASK-2291'
modified_files:
  - crates/cli/src/args.rs
  - crates/cli/src/main.rs
priority: high
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: `ops lock <name> -- <cmd…>` runs a command while holding a named repository lock, and releases it on every exit path, signals included. Also `ops lock status [<name>]` (holder PID, host, worktree, command, age, whether the holder is still alive) and `ops lock break <name>` (only when the holder is provably dead, unless `--force`). Locks live under the common git dir, so every worktree of one repository shares them.

**Why**: the `code-review-run-wave` / `code-review-run-waves` skills serialize merges and backlog bookkeeping with `until mkdir .git/code-review-merge.lock; do sleep 1; done` plus `trap 'rmdir …' EXIT`. A killed runner leaves the lock directory behind. The protocol has a manual recovery section for it (check that no rebase is in progress in any worktree, then `rmdir`), and the fan-out runner has to check by hand that the lock does not exist. The lock records nothing about who holds it.

**Used by**: `code-review-run-wave`, `code-review-run-waves`.
Source: skills-vs-ops audit of rsvalerio/ai dev-skills, 2026-09-26 (https://claude.ai/artifact/4yaKVkPdfe93hrqhFW5u1z).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `ops lock <name> -- <cmd>` acquires, runs and releases, including on SIGINT/SIGTERM and non-zero exit
- [x] #2 `ops lock status` reports holder PID, worktree, command, age and liveness; a dead holder is reported as stale
- [x] #3 Locks are shared across git worktrees of one repository (stored under the common git dir)
- [x] #4 Tests cover contention, a stale holder, and release on failure

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented as crates/cli/src/lock_cmd.rs: flock(2) on <git-common-dir>/ops/locks/<name>.lock (kernel drops the lock on any holder death, so no lock can outlive its holder); record (pid, host, worktree, command, acquired) written after locking, cleared on release; SIGINT/SIGTERM forwarded to the child and ops exits 128+signo after it. `ops lock break` clears a dead holder record; it has no --force: a live holder owns a kernel lock that cannot be revoked from outside, so the refusal names the holder to stop instead. Tests: lock_cmd unit tests (contention, stale record, release on failure) + crates/cli/tests/lock.rs (shared across worktrees, SIGTERM release, failing command, SIGKILL -> stale).
<!-- SECTION:NOTES:END -->
