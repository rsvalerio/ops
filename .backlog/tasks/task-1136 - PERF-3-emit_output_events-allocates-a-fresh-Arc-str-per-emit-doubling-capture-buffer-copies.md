---
id: TASK-1136
title: >-
  PERF-3: emit_output_events allocates a fresh Arc<str> per emit, doubling
  capture-buffer copies
status: Done
assignee:
  - TASK-1263
created_date: '2026-05-08 07:40'
updated_date: '2026-05-09 11:00'
labels:
  - code-review-rust
  - PERF
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:239-278`

**What**: Function takes `stdout: &str` / `stderr: &str` and constructs `Arc::<str>::from(output)` inside, copying the bytes. The caller already owns `String` buffers in `CommandOutput.stdout/stderr` and copies them into a new `Arc<str>`, while `build_step_result` consumes the original `String`. Two paths each pay one full-buffer copy.

**Why it matters**: With `OPS_OUTPUT_BYTE_CAP` defaulting to 4 MiB and `MAX_PARALLEL=32`, up to ~256 MiB extra copy traffic per plan. PERF-3/TASK-0732/0838 design explicitly intended one allocation per stream.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Pass captured strings by value or as Arc<str> into emit_output_events so output is wrapped exactly once
- [x] #2 StepResult.stdout/stderr continue to surface owned String (or migrate to Arc<str> as a follow-up)
- [x] #3 Test confirms one alloc per stream
<!-- AC:END -->
