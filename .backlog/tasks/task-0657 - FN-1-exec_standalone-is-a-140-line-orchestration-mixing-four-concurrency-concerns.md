---
id: TASK-0657
title: >-
  FN-1: exec_standalone is a 140-line orchestration mixing four concurrency
  concerns
status: To Do
assignee: []
created_date: '2026-04-30 05:12'
updated_date: '2026-04-30 06:07'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:316-455`

**What**: `exec_standalone` mixes four concerns: abort short-circuit, per-task channel + forwarder spawn, terminal-event capture, dropped-output reporting. Each concern is well-commented but the function has four nested control-flow regimes (abort guard, forwarder spawn, exec_command callback, terminal drain) and has accreted four CONC-* / ERR-* annotations.

**Why it matters**: A change to any one regime needs to re-read the others to confirm invariants. FN-1 single-abstraction-level guideline applies.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract spawn_event_forwarder(local_rx, outer_tx, abort) -> JoinSet so the forwarder spawn-and-drain is one call
- [ ] #2 Extract forward_terminal_event_or_drop(tx, ev, abort) for the trailing tokio::select! block
<!-- AC:END -->
