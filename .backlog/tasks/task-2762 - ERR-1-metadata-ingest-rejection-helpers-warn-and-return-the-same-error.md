---
id: TASK-2762
title: 'ERR-1: metadata ingest rejection helpers warn and return the same error'
status: Done
assignee: []
created_date: '2026-11-18 18:42'
updated_date: '2026-11-18 21:47'
labels:
  - code-review-rust
  - ERR
dependencies: []
parent_task_id: 'TASK-2853'
modified_files:
  - extensions-rust/metadata/src/ingestor.rs
priority: low
ordinal: 1000
dedup_key: 'ERR-1:extensions-rust/metadata/src/ingestor.rs:reject_non_singleton,reject_non_object'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/ingestor.rs:233`, `extensions-rust/metadata/src/ingestor.rs:254`

**What**: Two functions log a breach at warn level and then build the error that the caller propagates. Both describe the same event and include the same "dropping the table so the next run re-ingests" remediation:
- `reject_non_singleton` (ingestor.rs:232-243) warns "metadata_raw must hold exactly one workspace_root row" and returns `external_err("metadata_raw must contain exactly one row, found {n}; dropped metadata_raw ...")`.
- `reject_non_object` (ingestor.rs:253-264) warns "metadata_raw payload must be a single JSON object" and returns `external_err("... must be a single JSON object, found {observed}; dropped metadata_raw ...")`.

**Why it matters**: ERR-1 says to log only at the handling site. The error travels through `validate_published` and `load` to the orchestrator, which reports it, so each rejection is logged twice. The warn text ("exactly one workspace_root row") also differs from the error text ("exactly one row"), so the two entries do not obviously correlate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 reject_non_singleton and reject_non_object return their error without a duplicate warn event for the same breach
- [x] #2 The returned error still names the observed row count or shape and states that metadata_raw was dropped

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.18 (code-review-rust run, 2026-11-18).

TASK-2853: warns removed. The rejection no longer drops the table itself: it propagates out of the load transaction, which rolls back. The error therefore says the metadata_raw load was rolled back (the accurate statement) rather than dropped; AC #2 intent (state that no metadata_raw remains) is met by that wording.

<!-- SECTION:NOTES:END -->
