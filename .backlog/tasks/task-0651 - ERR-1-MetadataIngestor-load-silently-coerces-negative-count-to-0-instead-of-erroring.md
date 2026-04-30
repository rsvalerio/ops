---
id: TASK-0651
title: >-
  ERR-1: MetadataIngestor::load silently coerces negative count(*) to 0 instead
  of erroring
status: Done
assignee:
  - TASK-0738
created_date: '2026-04-30 05:01'
updated_date: '2026-04-30 18:28'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/ingestor.rs:56-61`

**What**: After `query_row("SELECT count(*) FROM metadata_raw")` returns an `i64`, the code calls `.map(|c| u64::try_from(c).unwrap_or(0))`. A negative i64 (DuckDB anomaly, signed-overflow regression, or schema drift to a SUM-style aggregate over a virtual table) is silently converted to 0 and stored in `data_sources.record_count`.

The sibling code in `extensions/duckdb/src/ingestor.rs::count_records_with` (lines 171-181) was hardened by TASK-0201/TASK-0506 to surface `DbError::InvalidRecordCount` on the same condition. The metadata ingestor diverged when TASK-0606 extended it to query an actual count instead of hard-coding `1`.

**Why it matters**: Drift from the project policy of making conversion failures explicit. An operator would see "load succeeded with 0 rows" instead of the typed error the rest of the codebase enforces, masking schema drift or DB corruption. Same anti-pattern previously closed elsewhere in TASK-0506.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace `unwrap_or(0)` at extensions-rust/metadata/src/ingestor.rs:60 with `u64::try_from(raw_count).map_err(|_| DbError::InvalidRecordCount { table: "metadata_raw", count: raw_count })`, mirroring extensions/duckdb/src/ingestor.rs:181
- [ ] #2 Add a unit test exercising the negative-count branch, paralleling `negative_record_count_surfaces_as_invalid_record_count_error` at extensions/duckdb/src/ingestor.rs:520
- [ ] #3 Grep workspace for any remaining `u64::try_from(.*)\.unwrap_or(0)` patterns over DB counts and confirm none remain
<!-- AC:END -->
