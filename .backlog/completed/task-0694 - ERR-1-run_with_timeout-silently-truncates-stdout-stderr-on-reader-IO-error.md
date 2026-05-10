---
id: TASK-0694
title: 'ERR-1: run_with_timeout silently truncates stdout/stderr on reader IO error'
status: Done
assignee:
  - TASK-0735
created_date: '2026-04-30 05:26'
updated_date: '2026-04-30 06:14'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/subprocess.rs:177-215`

**What**: The two pipe-drain threads spawned in `run_with_timeout` each call `let _ = s.read_to_end(&mut buf)`, discarding any IO error mid-read. The thread then returns its partial buffer, which the caller treats as the complete captured output. Combined with the timeout-path `let _ = stdout_handle.and_then(|h| h.join().ok())`, both an OS-level pipe error and a join panic silently produce truncated stdout/stderr that flows into `Output { stdout, stderr }` indistinguishably from a successful drain.

**Why it matters**: Callers in `extensions-rust/*` (cargo metadata/update/upgrade/deny) parse this stdout/stderr to make decisions (advisory severity, dependency lists). A truncated buffer round-trips as "valid but empty" data and produces wrong reports without any error path to log against.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Capture and surface read_to_end errors from stdout/stderr drain threads
- [x] #2 Distinguish a failed drain from an empty stream in RunError or in a tracing::warn event
- [x] #3 Keep the timeout-kill path resilient (still kill the child even if drain failed)
<!-- AC:END -->
