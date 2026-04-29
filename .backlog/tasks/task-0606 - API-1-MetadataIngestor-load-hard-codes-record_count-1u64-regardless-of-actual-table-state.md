---
id: TASK-0606
title: >-
  API-1: MetadataIngestor::load hard-codes record_count = 1u64 regardless of
  actual table state
status: Done
assignee:
  - TASK-0645
created_date: '2026-04-29 05:19'
updated_date: '2026-04-29 17:47'
labels:
  - code-review-rust
  - API
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/ingestor.rs:57`

**What**: record_count = 1u64 is a literal, not a `SELECT count(*) FROM metadata_raw`. If a future schema variant produces multiple rows, the upserted data_sources row reports 1 regardless of reality. Variable name implies it`s queried; it is invented.

**Why it matters**: Diagnostic data drift — data_sources.record_count is read by other tooling to decide if ingest looks healthy. Hard-coded 1 makes that signal lying.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 record_count is queried from the table, OR field is renamed/commented to make singleton invariant explicit
<!-- AC:END -->
