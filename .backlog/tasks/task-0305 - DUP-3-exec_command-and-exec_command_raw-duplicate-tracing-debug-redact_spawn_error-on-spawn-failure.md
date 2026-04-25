---
id: TASK-0305
title: >-
  DUP-3: exec_command and exec_command_raw duplicate tracing-debug +
  redact_spawn_error on spawn failure
status: Done
assignee:
  - TASK-0323
created_date: '2026-04-24 08:52'
updated_date: '2026-04-25 12:30'
labels:
  - rust-code-review
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: crates/runner/src/command/exec.rs:208-217 and :260-264

**What**: Both paths repeat tracing::debug!(error = %e, program = %spec.program, …) followed by redact_spawn_error(&spec.program, &e).

**Why it matters**: Drift risk if redaction/tracing fields evolve (SEC-21 relevance). Small but exactly the DUP-3 pattern.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Extract a log_and_redact_spawn_error helper
- [x] #2 Both call sites use the helper; existing tests pass unchanged
<!-- AC:END -->
