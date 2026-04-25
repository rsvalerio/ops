---
id: TASK-0300
title: >-
  CONC-5: crates/core/src/subprocess.rs uses thread::sleep + try_wait polling
  loop unsafe from async callers
status: Done
assignee:
  - TASK-0323
created_date: '2026-04-24 08:52'
updated_date: '2026-04-25 12:21'
labels:
  - rust-code-review
  - concurrency
  - async
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/subprocess.rs:106-146`

**What**: `run_with_timeout` uses blocking `thread::sleep` + `try_wait` polling (100ms) and synchronous `Read` on stdout/stderr pipes via `spawn`ed OS threads. Invoking this from an async context blocks the tokio runtime for the duration of poll cycles and adds up to 100ms of timeout latency.

**Why it matters**: The helper is now shared across extensions that may run under tokio tasks; a blocking sleep in the runtime thread pool can starve other tasks and defeats the async timeout story.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 All async callers wrap invocations in spawn_blocking OR a tokio-based async variant is introduced
- [x] #2 Poll interval replaced with wait-based primitive (e.g. tokio::process::Child::wait_with_output + timeout) or the 100ms cadence is explicitly documented as the resolution ceiling
<!-- AC:END -->
