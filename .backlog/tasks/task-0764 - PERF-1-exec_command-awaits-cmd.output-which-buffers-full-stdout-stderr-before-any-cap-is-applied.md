---
id: TASK-0764
title: >-
  PERF-1: exec_command awaits cmd.output() which buffers full stdout/stderr
  before any cap is applied
status: Done
assignee:
  - TASK-0824
created_date: '2026-05-01 05:55'
updated_date: '2026-05-01 09:44'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:229`

**What**: tokio::process::Command::output() reads both pipes to completion into Vec<u8> before returning. CommandOutput::from_raw then truncates to OPS_OUTPUT_BYTE_CAP (4 MiB / stream default). A misbehaving child writing 1 GiB to stdout still allocates 1 GiB before truncation.

**Why it matters**: Defeats SEC-33 / PERF-1 intent. Real peak is unbounded relative to child output; only stored result is capped. Under MAX_PARALLEL=32 misbehaving siblings can cumulatively exhaust memory before truncation runs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Stream stdout/stderr (e.g. via cmd.stdout(Stdio::piped()).spawn() + tokio AsyncRead loop) and stop reading after cap bytes per stream, draining the rest to /dev/null without buffering
- [x] #2 Regression test piping >cap bytes asserts peak RSS / collected bytes stays bounded near cap
- [x] #3 Update DEFAULT_OUTPUT_BYTE_CAP doc to describe streaming behaviour rather than implying truncate-on-store
<!-- AC:END -->
