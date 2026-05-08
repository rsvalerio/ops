---
id: TASK-1147
title: >-
  ARCH-1: run-before-commit/lib.rs is 702 lines with hook-script, env clamp,
  subprocess orchestration, typed errors
status: To Do
assignee:
  - TASK-1264
created_date: '2026-05-08 07:42'
updated_date: '2026-05-08 13:18'
labels:
  - code-review-rust
  - ARCH
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:1`

**What**: Crate root mixes impl_extension boilerplate, HOOK_SCRIPT, the env-driven timeout policy (git_timeout_from_env, MAX_GIT_TIMEOUT_SECS, clamp WARN logging), bounded-wait git diff orchestrator (has_staged_files_with_timeout, read_stderr_bounded, STDERR_DRAIN_GRACE), and HasStagedFilesError. None of the subprocess machinery is in ops_hook_common despite the two hook crates explicitly factoring shared shape via impl_hook_wrappers!.

**Why it matters**: A future hook needing the same bounded-wait shape (pre-merge-commit, prepare-commit-msg) will copy these 200+ lines. The crate is doing two unrelated things: extension wiring and a generic git-state probe.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Move has_staged_files_with_timeout, read_stderr_bounded, HasStagedFilesError, and git_timeout_from_env into ops_hook_common::git_state
- [ ] #2 Keep a thin pub re-export in run-before-commit::lib so existing call sites compile unchanged
<!-- AC:END -->
