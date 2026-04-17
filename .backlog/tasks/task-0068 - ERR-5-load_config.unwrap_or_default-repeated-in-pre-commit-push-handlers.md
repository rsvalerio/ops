---
id: TASK-0068
title: 'ERR-5: load_config().unwrap_or_default() repeated in pre-commit/push handlers'
status: Done
assignee: []
created_date: '2026-04-17 11:30'
updated_date: '2026-04-17 15:41'
labels:
  - rust-codereview
  - err
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/main.rs:458`

**What**: `run_before_commit` and `run_before_push` both use `load_config().unwrap_or_default()` when deciding whether to prompt install, hiding config errors.

**Why it matters**: A parse failure in .ops.toml would make ops claim no command is configured and offer to install — potentially overwriting user intended config.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Propagate config load errors or warn!() on failure before falling back
- [ ] #2 Cover failure mode with a test demonstrating user sees the error
<!-- AC:END -->
