---
id: TASK-0510
title: >-
  ERR-1: MetadataIngestor::load fails entire load when remove_file cleanup
  errors
status: Done
assignee:
  - TASK-0533
created_date: '2026-04-28 06:51'
updated_date: '2026-04-28 18:00'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/ingestor.rs:73`

**What**: load() successfully ingests metadata then calls std::fs::remove_file(&path)?. If cleanup fails (read-only mount, AV race), the entire load is reported as failure even though the DuckDB row is committed.

**Why it matters**: Subsequent invocations would see the row already loaded but treat the previous attempt as failed, possibly retrying ingestion. Cleanup errors should be logged at warn and ignored.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 remove_file failure is logged at warn but does not propagate
- [x] #2 load returns LoadResult::success even when cleanup fails
<!-- AC:END -->
