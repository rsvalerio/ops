---
id: TASK-0447
title: >-
  ERR-1: init_logging swallows OPS_LOG_LEVEL parse error to debug before
  subscriber registers
status: Done
assignee:
  - TASK-0536
created_date: '2026-04-28 05:43'
updated_date: '2026-04-28 16:14'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/main.rs:71-95`

**What**: `tracing::debug!` inside `init_logging` is emitted before the subscriber is registered, so the diagnostic is lost. The user-facing fallback to INFO happens silently.

**Why it matters**: A typo like `OPS_LOG_LEVEL=debg` is invisible. Users debugging "why no debug logs?" get no signal that their env var was rejected.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Print invalid OPS_LOG_LEVEL to stderr (via ops_core::ui::warn or eprintln!) before falling back, since tracing is not yet up
- [x] #2 Test or manual verification that an invalid value produces a visible warning
<!-- AC:END -->
