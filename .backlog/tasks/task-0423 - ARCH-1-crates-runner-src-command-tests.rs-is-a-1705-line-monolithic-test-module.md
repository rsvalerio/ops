---
id: TASK-0423
title: >-
  ARCH-1: crates/runner/src/command/tests.rs is a 1705-line monolithic test
  module
status: To Do
assignee:
  - TASK-0537
created_date: '2026-04-28 04:41'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/tests.rs:1` (1705 lines)

**What**: Single test file in the runner command module mixes resolve/expand, sequential-plan, parallel-plan, exec, secret-pattern, and event-emission tests with no submodule split. Same pattern as the already-split TASK-0353 (theme/src/tests.rs).

**Why it matters**: 1705 lines exceeds ARCH-1's 500-line module red flag; locating tests for a given concern requires scrolling, and edits routinely conflict at merge time.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Split into per-concern submodules under crates/runner/src/command/tests/ (e.g. expand.rs, parallel.rs, sequential.rs, exec.rs, events.rs)
- [ ] #2 Top-level tests.rs contains only mod declarations and any shared fixtures
- [ ] #3 No file in the new layout exceeds ~500 lines
<!-- AC:END -->
