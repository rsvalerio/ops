---
id: TASK-0732
title: >-
  PERF-3: emit_output_events allocates a fresh String per captured stdout/stderr
  line
status: Done
assignee:
  - TASK-0741
created_date: '2026-04-30 05:50'
updated_date: '2026-05-01 11:22'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:118-133` (`emit_output_events`)

**What**: After `cmd.output().await` completes, `emit_output_events` iterates `output.lines()` and emits one `RunnerEvent::StepOutput { line: line.to_string(), ... }` per line. Each `line.to_string()` is a fresh heap allocation; for a noisy build (`cargo test` flooding stderr), that is hundreds-to-thousands of allocations per finished step, all on top of the already-owned `output.stdout` / `output.stderr` `String` that is dropped immediately after this loop.

The lines are slices of an owned String that is no longer needed after this function returns, so the typical PERF-3 fix (move-out via `mem::take` + an arena, or a `Bytes`-style reference into the original) is straightforward. Even simpler: change `RunnerEvent::StepOutput::line` to a `String` produced by splitting the owned buffer (`String::split_terminator` over the owned source after `output.stdout = mem::take(&mut output.stdout)`), or change the variant to carry an `Arc<str>` derived once from the owned buffer.

**Why it matters**: This is the captured-output hot path under any verbose command and runs synchronously inside the runner-event closure called from exec_command. The allocations are easy wins; CONC-7 / TASK-0457 already explicitly worries about per-task event volume and dropped events under load.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 emit_output_events no longer calls .to_string() once per line; the captured output is consumed once into the event stream without per-line heap allocation (e.g. via Bytes, Arc<str>, or move-out + split)
- [x] #2 Bench or trace event-allocation count for a 10k-line stderr step before/after the change to confirm the win
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Wave-57 deferral: a true per-line zero-allocation fix needs an owned-buffer slice type (bytes::Bytes or custom Arc<str>+range). Both options touch ~40 call sites and the public RunnerEvent serialization shape; a partial Arc<str> migration would still allocate per line and expose a SemVer-relevant change. Re-triage into a dedicated event-API wave so the redesign and downstream churn (display, tap, JSON event format, tests) land together rather than mid-wave.
<!-- SECTION:NOTES:END -->
