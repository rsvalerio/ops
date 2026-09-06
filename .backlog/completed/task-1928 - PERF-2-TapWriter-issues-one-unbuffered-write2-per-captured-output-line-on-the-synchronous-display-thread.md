---
id: TASK-1928
title: >-
  PERF-2: TapWriter issues one unbuffered write(2) per captured output line on
  the synchronous display thread
status: Done
assignee:
  - TASK-1986
created_date: '2026-08-27 15:46'
updated_date: '2026-08-28 19:14'
labels:
  - code-review-rust
  - performance
dependencies: []
modified_files:
  - crates/runner/src/display/tap.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display/tap.rs:20-32` (`TapWriter.file: Option<File>`), `crates/runner/src/display/tap.rs:57-78` (`write_line`), reached per line from `crates/runner/src/display.rs:549-555` (`on_step_output`)

**What**: `TapWriter` holds a bare `std::fs::File` and writes each line straight to it:

```rust
if let Some(ref mut f) = self.file {
    if let Err(e) = writeln!(f, "{line}") { ... }
}
```

`std::fs::File` has no buffering — `impl Write for File` forwards every call to the syscall. `writeln!` on it emits the fragments of the format (the line, then the newline), so a tap-enabled run pays at least one `write(2)` per captured output line, plus a second for the newline.

The call site makes that per-line, not per-step: `on_step_output` taps **every** `StepOutput` event, stdout as well as stderr, and `emit_output_events` (`exec.rs:263-319`) emits one such event per newline in the capture buffer — up to `OPS_OUTPUT_BYTE_CAP` / average-line-length events per stream. A `cargo build --all-targets` step easily produces tens of thousands; a 4 MiB capture at 60 bytes a line is ~70k events, so ~140k syscalls for one step.

Those syscalls land on the synchronous display thread, which is the same thread as the event pump — `handle_event` is documented as blocking-I/O-only and structurally pinned `!Send` for that reason (`display.rs:249-262`, CONC-5 / TASK-0331). Every tap write is therefore backpressure on the mpsc drain, which in turn is backpressure on the `exec_standalone` forwarders, which is what makes the per-task 256-slot buffer overflow and produce the `StepOutputDropped` events the crate already has machinery to report (CONC-7 / TASK-0457). In other words, the unbuffered tap is a plausible contributor to the dropped-output condition the display exists to warn about.

A `BufWriter<File>` reduces this to one syscall per ~8 KiB. It needs a matching flush discipline, which the current design does not have: `write_line` drops the handle on first error to disable further writes, `report_tap_truncation` (`display/finalize.rs:280-291`) drains the truncation record at `RunFinished`, and `append_marker` re-opens the path. With buffering, an unflushed tail would be lost whenever the process exits without a clean `RunFinished` — so an explicit flush at `RunFinished` plus a `Drop` impl is part of the fix, not an optional extra.

**Why it matters**: the tap file is the CI-facing artifact, so it is enabled precisely on the noisy, long, automated runs where the syscall cost is largest and where the display thread stalling turns into dropped output lines that explain a failure.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 TapWriter wraps the tap file in a buffered writer so a noisy step no longer costs one syscall per captured line
- [x] #2 buffered content is flushed at RunFinished and in a Drop impl, so an interrupted run cannot silently lose the tail of the tap
- [x] #3 the existing first-error-disables-tap and append_marker behaviours still hold with buffering in place, including the StorageFull / BrokenPipe short-circuit (TASK-1176)
- [x] #4 a test writes many lines through TapWriter and asserts the full content is present on disk after the flush points, not only after process exit
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
TapWriter.file is now Option<BufWriter<File>> (~one syscall per 8 KiB instead of one per captured line on the synchronous display thread). Flush discipline added: TapWriter::flush() is called from report_tap_truncation at RunFinished — before draining the truncation record, so a deferred spill failure is still reported in the same run — and from a new Drop impl for runs that never reach RunFinished. Write and flush failures share record_failure(), preserving first-error-disables-tap with the step id and ErrorKind; append_marker flushes before re-opening the path so the marker cannot precede the lines it marks, and the StorageFull/BrokenPipe short-circuit (TASK-1176) is untouched. Tests: buffered_lines_reach_disk_at_the_explicit_flush_point (2000 lines, asserted complete after flush while the writer is alive, then a post-flush line asserted after Drop) and first_write_failure_still_disables_the_tap_once.
<!-- SECTION:NOTES:END -->
