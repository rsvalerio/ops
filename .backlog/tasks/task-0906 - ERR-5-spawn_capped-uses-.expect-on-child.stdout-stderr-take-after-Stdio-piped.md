---
id: TASK-0906
title: >-
  ERR-5: spawn_capped uses .expect on child.stdout/stderr take after
  Stdio::piped
status: Triage
assignee: []
created_date: '2026-05-02 10:09'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:111`

**What**: After cmd.stdout(Stdio::piped()).stderr(Stdio::piped()) and a successful spawn, the code does `child.stdout.take().expect("stdout piped")` (likewise stderr). A future refactor moving the stdio configuration upward (or splitting build_command_async to accept partially-configured commands) will silently regress to a runtime panic instead of a StepFailed event.

**Why it matters**: ERR-5 / TASK-0456 already replaced expect()s in build_command_async for the same reason; this re-introduces the pattern in the parallel hot path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace .expect with a typed io::Error::other on None, routed through the existing log_and_redact_spawn_error path
- [ ] #2 Add a debug_assert to keep the invariant visible without panicking in release
<!-- AC:END -->
