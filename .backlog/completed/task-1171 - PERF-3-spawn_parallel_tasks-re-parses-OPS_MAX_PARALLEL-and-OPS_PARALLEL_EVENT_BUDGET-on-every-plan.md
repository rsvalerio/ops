---
id: TASK-1171
title: >-
  PERF-3: spawn_parallel_tasks re-parses OPS_MAX_PARALLEL and
  OPS_PARALLEL_EVENT_BUDGET on every plan
status: Done
assignee:
  - TASK-1262
created_date: '2026-05-08 08:06'
updated_date: '2026-05-08 15:36'
labels:
  - code-review-rust
  - perf
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/parallel.rs:207`

**What**: Every call to `spawn_parallel_tasks` invokes `resolve_max_parallel()` and `resolve_event_budget()`, each of which calls `std::env::var(...)` and `parse::<usize>()`. Both env vars are documented as process-global; only `OPS_OUTPUT_BYTE_CAP` (results.rs:137) is memoized via `OnceLock`.

**Why it matters**: Inconsistent with the sibling cap's OnceLock memoization and re-acquires the global env lock on every parallel plan. Under a server-style embedder running many plans, this contends with command-spawn env reads and re-emits the warn-on-fallback diagnostic per call.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Both resolvers cache their parsed value (and fallback warn fired exactly once) via OnceLock or equivalent, mirroring output_byte_cap.
- [ ] #2 A regression test mutates the env between two resolve_max_parallel() calls and asserts the second observes the first's value (memoized).
<!-- AC:END -->
