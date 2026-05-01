---
id: TASK-0768
title: >-
  ERR-1: finalize_orphan_bars renders Skipped state without emitting a
  StepSkipped RunnerEvent
status: Triage
assignee: []
created_date: '2026-05-01 05:55'
labels:
  - code-review-rust
  - errors
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display/finalize.rs:28-44`

**What**: When a parallel run is aborted mid-flight, bars whose tasks never produced a terminal event are finalized in display by directly mutating bar state. No RunnerEvent::StepSkipped is emitted on the event stream, so JSON event consumers see those steps stuck in StepStarted state forever.

**Why it matters**: Display and event-stream views diverge — TTY shows three skipped rows; an external consumer subscribed to RunnerEvent sees one StepStarted with no terminal event. Per-step terminal status is missing for orphans, defeating the typed event API.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Emit RunnerEvent::StepSkipped { id, display_cmd: None } for each orphan before the bar is finalized, going through the standard on_event callback path
- [ ] #2 Test runs a fail_fast parallel plan with a slow sibling, captures all RunnerEvents, asserts every started step has a matching terminal (Finished/Failed/Skipped) event
- [ ] #3 Keep the visual rendering identical (still uses StepStatus::Skipped + the bar elapsed)
<!-- AC:END -->
