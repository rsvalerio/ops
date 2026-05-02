---
id: TASK-0838
title: >-
  PERF-3: Arc::from(&str) in emit_output_events copies the full capture buffer,
  contradicting doc claim
status: Triage
assignee: []
created_date: '2026-05-02 09:13'
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
- [ ] #1 Either change emit_output_events to take String (or Arc<str>) by value and consume output without re-allocating, OR correct the doc-comment to reflect that the cost is one buffer-copy per stream
- [ ] #2 Add a regression assertion (e.g. via Arc::strong_count plus Arc::ptr_eq) that proves the chosen ownership model
- [ ] #3 Re-measure the per-step allocation count under the OPS_OUTPUT_BYTE_CAP=4194304 worst case
<!-- AC:END -->
