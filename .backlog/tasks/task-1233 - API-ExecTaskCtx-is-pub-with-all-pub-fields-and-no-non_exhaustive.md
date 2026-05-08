---
id: TASK-1233
title: 'API: ExecTaskCtx is pub with all-pub fields and no #[non_exhaustive]'
status: To Do
assignee:
  - TASK-1269
created_date: '2026-05-08 12:58'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - api
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:473-483`

**What**: `ExecTaskCtx` is exposed as pub from `ops_runner::command` (used by pub `exec_standalone`). Every field is pub and the struct lacks `#[non_exhaustive]`, so any future addition (per-task telemetry, cancellation token, budget) becomes a SemVer-breaking change for downstream embedders that construct it via struct-literal syntax.

**Why it matters**: The runner is consumed by the CLI plus likely embedders. The struct's role as a task context bag is exactly the shape API-9 says should be #[non_exhaustive] from day one. RunnerEvent and StepResult already are; this is the gap.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add #[non_exhaustive]
- [ ] #2 Provide a new(cwd, vars, tx, abort, policy) constructor
- [ ] #3 Document the stability contract in the type's rustdoc
<!-- AC:END -->
