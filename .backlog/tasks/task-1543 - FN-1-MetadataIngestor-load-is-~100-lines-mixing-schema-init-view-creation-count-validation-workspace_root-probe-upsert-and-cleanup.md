---
id: TASK-1543
title: >-
  FN-1: MetadataIngestor::load is ~100 lines mixing schema init, view creation,
  count validation, workspace_root probe, upsert, and cleanup
status: Done
assignee:
  - TASK-1576
created_date: '2026-05-19 15:24'
updated_date: '2026-05-19 17:48'
labels:
  - code-review-rust
  - FN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/ingestor.rs:43-146`

**What**: `MetadataIngestor::load` runs ~104 source lines and threads six distinct responsibilities through one function body:
1. `init_schema(db)` + acquire connection (lines 44-45)
2. Build `metadata_raw` table via `views::metadata_raw_create_sql` (lines 47-50)
3. Create the `crate_dependencies` view (lines 52-54)
4. Query and validate `record_count` with `InvalidRecordCount` mapping (lines 62-72)
5. Multi-row warn + `workspace_root` extraction with a fallback `typeof(...)` probe on error (lines 84-116)
6. Checksum, upsert into `data_sources`, and best-effort `remove_file` cleanup (lines 120-143)

Step 5 alone is a 33-line block with an inline closure that itself runs a secondary DuckDB query just to enrich the error message. The function exceeds FN-1's 50-line guideline by ~2x and the cognitive scope mixes three nesting levels (closure inside `map_err` inside `query_row`).

**Why it matters**: Each step is independently testable and reviewable, but the current shape forces reviewers to hold the whole pipeline in their head to reason about any single stage. Extracting `build_views(&conn) -> DbResult<()>`, `query_record_count(&conn) -> DbResult<u64>`, `extract_workspace_root(&conn) -> DbResult<String>`, and `cleanup_staged_file(&path)` would let each call site read at one nesting level and let unit tests cover the workspace_root `typeof` probe directly. This pays off when (not if) a future ingest step gets bolted on.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 MetadataIngestor::load body is under 50 lines after extraction
- [ ] #2 The workspace_root typeof-probe fallback path is reachable from a dedicated unit test on the extracted helper
- [ ] #3 Existing tests (metadata_load_with_sample_data, metadata_load_warns_when_metadata_raw_has_multiple_rows, crate_dependencies_view_*) still pass without modification
<!-- AC:END -->
