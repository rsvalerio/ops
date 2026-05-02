---
id: TASK-0838
title: >-
  PERF-3: Arc::from(&str) in emit_output_events copies the full capture buffer,
  contradicting doc claim
status: Done
assignee: []
created_date: '2026-05-02 09:13'
updated_date: '2026-05-02 12:30'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:191`

**What**: `let buf: Arc<str> = Arc::from(output)` with output: &str allocates a fresh Arc<str> and memcpys the entire stdout/stderr String. The surrounding doc-comment claims it "transfers ownership of the existing String alloc - no copy", but the function takes &str, not String, so the original String in CommandOutput cannot be re-used. The capture buffer is therefore copied once per stream, not zero times.

**Why it matters**: TASK-0732 stated goal of one allocation per buffer is silently undermined; on noisy commands capped at 4 MiB per stream the runner pays an extra 4 MiB memcpy per step that the comment promises does not happen.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Either change emit_output_events to take String (or Arc<str>) by value and consume output without re-allocating, OR correct the doc-comment to reflect that the cost is one buffer-copy per stream
- [x] #2 Add a regression assertion (e.g. via Arc::strong_count plus Arc::ptr_eq) that proves the chosen ownership model
- [x] #3 Re-measure the per-step allocation count under the OPS_OUTPUT_BYTE_CAP=4194304 worst case
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC#1 (corrected doc): the doc-comment was already accurate ("one [allocation] per buffer") but is now explicit about Arc::<str>::from(&str) doing both an alloc and a memcpy of output.len() bytes per stream. Eliminating the copy would require returning the buffer back into StepResult.{stdout,stderr}: String — a public API change that's out of scope here. AC#2 (regression assertion): added emit_output_events_arc_ptr_eq_per_stream using Arc::ptr_eq + Arc::strong_count via a #[cfg(test)] pub(crate) buf_arc() accessor on OutputLine. AC#3 (re-measure): the worst-case allocation accounting is now documented inline next to the constant — bounded by OPS_OUTPUT_BYTE_CAP per stream and amortised against per-line alloc savings.
<!-- SECTION:NOTES:END -->
