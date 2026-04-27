---
id: TASK-0334
title: >-
  SEC-21: Panic payload from JoinError forwarded into StepResult.message
  verbatim
status: Done
assignee:
  - TASK-0414
created_date: '2026-04-26 09:33'
updated_date: '2026-04-26 10:22'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/parallel.rs:64-68`

**What**: `collect_join_results` constructs the failure message via `format!("task panicked: {}", e)` where e is a JoinError whose Display includes the panic payload. Panic messages can contain attacker-influenced data including absolute paths.

**Why it matters**: That message flows into StepResult.message → StepFailed event → tap file / TAP CI output, exactly the leak channel SEC-22 redaction in exec.rs:98-103 was added to close on the spawn-error path. The panic path is asymmetric.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Redact or downgrade JoinError payloads similarly to redact_spawn_error: log full payload at debug, surface a generic message plus task id
- [ ] #2 Test asserts that a child task panicking with a path-containing message does not surface that path in StepResult.message
<!-- AC:END -->
