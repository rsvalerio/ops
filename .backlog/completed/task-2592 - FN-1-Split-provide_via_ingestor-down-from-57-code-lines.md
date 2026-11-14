---
id: TASK-2592
title: 'FN-1: Split provide_via_ingestor down from ~57 code lines'
status: Done
assignee: []
created_date: '2026-10-10 15:47'
updated_date: '2026-10-10 21:56'
labels:
  - code-review-rust
  - fn
dependencies: []
parent_task_id: 'TASK-2625'
modified_files:
  - extensions/sqlite/src/sql/ingest/orchestrator.rs
priority: medium
ordinal: 1000
dedup_key: 'FN-1:extensions/sqlite/src/sql/ingest/orchestrator.rs:provide_via_ingestor'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/sqlite/src/sql/ingest/orchestrator.rs:75-169`

**What**: `provide_via_ingestor` runs roughly 57 code lines between signature and closing brace, over the FN-1 guidance of at most 50. The body sequences four distinct concerns inline: in-memory-handle rejection, per-table ingest-mutex acquisition with poison recovery, the sidecar collect/checksum/load pipeline call, and the error re-wrap pass.

**Why it matters**: Long functions hide structure: each inline concern here is a state transition a reviewer must re-derive from the whole body. The clearest extraction is the trailing error-normalisation block (the loop that re-wraps pipeline failures into `DbError::External` with typed payload preservation, ~lines 147-160): lifting it into a named helper (e.g. `normalise_pipeline_errors`) brings the function under the limit and gives the re-wrap policy a name that the error.rs docs can point at.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 provide_via_ingestor is at most 50 code lines, with the DbError::External re-wrap block extracted into a named helper that preserves the existing source-chain behaviour

<!-- AC:END -->
