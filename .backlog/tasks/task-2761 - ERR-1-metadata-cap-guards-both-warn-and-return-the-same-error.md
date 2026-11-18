---
id: TASK-2761
title: 'ERR-1: metadata cap guards both warn and return the same error'
status: To Do
assignee: []
created_date: '2026-11-18 18:41'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - ERR
dependencies: []
parent_task_id: 'TASK-2853'
modified_files:
  - extensions-rust/metadata/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'ERR-1:extensions-rust/metadata/src/lib.rs:check_metadata_not_capped,query_metadata_raw_with_cap'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/lib.rs:108`, `extensions-rust/metadata/src/lib.rs:389`

**What**: Two functions emit a `tracing::warn!` and then return an error that describes the same failure:
- `check_metadata_not_capped` (lib.rs:108-120) warns "cargo metadata stdout hit the subprocess capture cap" and then `anyhow::bail!`s with the same cap, kept-bytes and env-var information.
- `query_metadata_raw_with_cap` (lib.rs:389-398) warns "metadata_raw payload exceeds byte cap" and then `anyhow::bail!`s with the same byte count, cap and env var.

**Why it matters**: ERR-1 says to handle or propagate an error, never both, and to log only at the handling site. Each error reaches the provider and orchestrator caller, which reports it. An over-cap workspace therefore produces two entries for one failure, and the warn is emitted even when a caller deliberately handles the error.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 check_metadata_not_capped and query_metadata_raw_with_cap return their error without also emitting a warn-level event for the same failure
- [ ] #2 Fields that exist only in the warn event (kept_bytes, cap, env) are preserved in the returned error message
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.18 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
