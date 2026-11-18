---
id: TASK-2686
title: 'API-14: run_with_timeout # Errors misstates the spawn-failure variant'
status: To Do
assignee: []
created_date: '2026-11-18 18:27'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - API
dependencies: []
parent_task_id: 'TASK-2840'
modified_files:
  - crates/core/src/subprocess/mod.rs
  - crates/core/src/subprocess/drain.rs
priority: low
ordinal: 1000
dedup_key: 'API-14:crates/core/src/subprocess/mod.rs:run_with_timeout'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/subprocess/mod.rs:177-180` (`run_with_timeout` doc), secondary: `crates/core/src/subprocess/drain.rs:43-46` (`read_capped` comment)

**What**: The `# Errors` section of `run_with_timeout` says "Returns [`RunError::Io`] if spawning or waiting on the child fails", but a spawn failure is mapped to `RunError::Spawn(SpawnError { .. })` in `run_with_timeout_inner`; the section also omits the documented-below `RunError::Io` cases for a panicked drain thread or a zero-byte drain failure. `run_cargo`'s doc gets it right ("Returns [`RunError::Spawn`] if the subprocess fails to spawn"), so the two public entry points disagree. Related stale comment: `read_capped` says the 64 KiB pre-size cap protects against "a huge cap (256 MiB default)" while `DEFAULT_OUTPUT_BYTE_CAP` is 4 MiB.

**Why it matters**: Callers matching on `RunError` to detect a missing binary follow the doc and match `Io`, which never fires for spawn failures.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 run_with_timeout's # Errors lists RunError::Spawn for spawn failure, RunError::Io for wait and drain failures, and RunError::Timeout
- [ ] #2 The read_capped comment cites the actual default cap (DEFAULT_OUTPUT_BYTE_CAP)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.03 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
