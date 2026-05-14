---
id: TASK-1431
title: >-
  ERR-1: ExecCommandSpec::validate accepts cwd containing '..' that escapes the
  workspace
status: Done
assignee:
  - TASK-1456
created_date: '2026-05-13 18:23'
updated_date: '2026-05-14 07:41'
labels:
  - code-review-rust
  - ERR
  - SEC
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/commands.rs:105`

**What**: `validate` checks `program` non-empty and `timeout_secs != 0` but ignores `cwd`. A `cwd = \"../../etc\"` (or symlink-laundered path) loads silently and only surfaces at exec time.

**Why it matters**: SEC-25 (task-1388) hardens atomic_write against symlinked destinations; cwd is the symmetric attack on `ops run <cmd>` invocations under a hostile workspace config. Validating at load gives operators an early, clear error.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Reject relative cwd containing '..' components at load time (or document the policy if intentional)
- [ ] #2 Regression test: workspace config with cwd='..' is rejected with a clear message
<!-- AC:END -->
