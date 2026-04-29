---
id: TASK-0589
title: 'ASYNC-6: has_staged_files_with spawns git subprocess with no bounded wait'
status: Triage
assignee: []
created_date: '2026-04-29 05:18'
labels:
  - code-review-rust
  - ASYNC
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:54`

**What**: `Command::new(program).args([...]).output()` blocks indefinitely waiting for git diff --cached. If git hangs (FUSE-backed worktree, network-mounted .git, lock contention), the pre-commit hook hangs the user`s commit forever with no diagnostic. No wait_timeout/kill-on-deadline.

**Why it matters**: A pre-commit hook is on the developer`s critical path. Hanging git is rare but not unheard of, and the hook stack has no escape valve.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 has_staged_files_with enforces a bounded wait (e.g. 5s default, configurable)
- [ ] #2 On timeout, function returns a typed error (not generic 'failed to run')
- [ ] #3 Test simulates a hanging fake-git binary and asserts timeout fires
<!-- AC:END -->
