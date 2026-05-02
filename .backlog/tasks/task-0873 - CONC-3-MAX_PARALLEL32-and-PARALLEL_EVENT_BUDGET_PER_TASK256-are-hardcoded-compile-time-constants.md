---
id: TASK-0873
title: >-
  CONC-3: MAX_PARALLEL=32 and PARALLEL_EVENT_BUDGET_PER_TASK=256 are hardcoded
  compile-time constants
status: Triage
assignee: []
created_date: '2026-05-02 09:23'
labels:
  - code-review-rust
  - concurrency
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/parallel.rs:24-33`

**What**: MAX_PARALLEL = 32 and PARALLEL_EVENT_BUDGET_PER_TASK = 256 are private consts. Operators on a 4-CPU CI box pay the same FD/process pressure as a 64-core developer, and the channel capacity (8192 events) is not adjustable per-workload.

**Why it matters**: A user running cargo ops verify with twenty composite parallel groups on a small CI runner will hit FD limits at 32 children with no way to dial it down. Exposing these via OutputConfig or env var (mirroring OPS_OUTPUT_BYTE_CAP) gives both populations a release valve.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Read MAX_PARALLEL from an env var (e.g. OPS_MAX_PARALLEL) or Config::output, with the current value as default
- [ ] #2 Same for PARALLEL_EVENT_BUDGET_PER_TASK (or derive it from the parallel cap)
- [ ] #3 Surface invalid values via the same tracing::warn! pattern proposed for OPS_OUTPUT_BYTE_CAP
<!-- AC:END -->
