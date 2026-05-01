---
id: TASK-0770
title: >-
  OWN-3: exec_command clones display_cmd up to four times per step on the
  success path
status: Triage
assignee: []
created_date: '2026-05-01 05:55'
labels:
  - code-review-rust
  - ownership
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:203`

**What**: `let display_cmd = Some(spec.display_cmd().into_owned())` then `display_cmd.clone()` for StepStarted, plus display_cmd (moved) into emit_step_completion which moves it again. On the success path the value is Option<String> carrying the same logical text used at start and finish.

**Why it matters**: Hot path on every spawn. Could be reduced to one Arc<str> shared between events, or pass &str and let the consumer allocate only when serializing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Refactor RunnerEvent to carry display_cmd as Option<Arc<str>> (or document the clone is intentional)
- [ ] #2 Or change emission to pass display_cmd by reference and clone at the JSON serialization boundary only
- [ ] #3 Keep public RunnerEvent serde shape unchanged for downstream JSON consumers
<!-- AC:END -->
