---
id: TASK-0905
title: >-
  PERF-3: OPS_OUTPUT_BYTE_CAP applies per-stream so peak RSS scales as 2 *
  MAX_PARALLEL * cap
status: Done
assignee: []
created_date: '2026-05-02 10:09'
updated_date: '2026-05-02 11:18'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/results.rs:112`

**What**: DEFAULT_OUTPUT_BYTE_CAP=4 MiB applies to each of stdout and stderr per spawn. Under MAX_PARALLEL=32 (TASK-0873) with both streams saturated, peak runner memory is 32 * 2 * 4 MiB = 256 MiB just for in-flight capture buffers — there is no global ceiling tying this to available RAM.

**Why it matters**: An end user setting OPS_OUTPUT_BYTE_CAP=64M on a parallel plan can OOM the runner without ever exceeding the documented per-stream contract.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Document the per-spawn-stream multiplier on DEFAULT_OUTPUT_BYTE_CAP and OPS_OUTPUT_BYTE_CAP env
- [ ] #2 Optionally introduce a global cap (semaphore on capture bytes) or warn when cap * 2 * MAX_PARALLEL exceeds a sensible RSS guard
<!-- AC:END -->
