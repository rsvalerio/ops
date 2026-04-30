---
id: TASK-0668
title: 'ERR-1: git_timeout_from_env silently ignores unparseable values and 0'
status: Done
assignee:
  - TASK-0737
created_date: '2026-04-30 05:13'
updated_date: '2026-04-30 17:55'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:88-94`

**What**: `git_timeout_from_env` silently filters out unparseable values and `0`, returning `None` so the caller falls back to `DEFAULT_GIT_TIMEOUT`. No `tracing::warn!` is emitted for malformed input.

**Why it matters**: A user who sets `OPS_RUN_BEFORE_COMMIT_GIT_TIMEOUT_SECS=10s` (or any non-numeric/zero value) gets the default 5s with no diagnostic — they will not realise their override was ignored. Sibling code in workspace.rs warns on similar misconfigurations.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Emit tracing::warn! when the env var is present but unparseable or 0, naming the env var and the offending value
- [ ] #2 Add a regression test that captures the warn (or at least pins the value-level fall-through)
<!-- AC:END -->
