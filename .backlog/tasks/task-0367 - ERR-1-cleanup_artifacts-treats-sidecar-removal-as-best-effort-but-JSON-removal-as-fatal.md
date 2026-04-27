---
id: TASK-0367
title: >-
  ERR-1: cleanup_artifacts treats sidecar removal as best-effort but JSON
  removal as fatal
status: To Do
assignee:
  - TASK-0421
created_date: '2026-04-26 09:37'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/ingestor.rs:172`

**What**: cleanup_artifacts propagates remove_file(json_path) errors as DbError::Io, while remove_workspace_sidecar is best-effort. Asymmetric handling means a transient permission error on the JSON file fails the whole ingest after data successfully loaded into DuckDB.

**Why it matters**: Caller sees Err even though ingestion succeeded; retry logic re-runs collect/load needlessly.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Treat the JSON removal as best-effort (log + continue) consistent with the sidecar policy, or document why they differ
- [ ] #2 Test demonstrates that load returns Ok(LoadResult) when json_path removal fails post-upsert
<!-- AC:END -->
