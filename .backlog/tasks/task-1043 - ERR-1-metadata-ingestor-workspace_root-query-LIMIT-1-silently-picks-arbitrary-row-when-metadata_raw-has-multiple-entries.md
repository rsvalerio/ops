---
id: TASK-1043
title: >-
  ERR-1: metadata ingestor 'workspace_root' query LIMIT 1 silently picks
  arbitrary row when metadata_raw has multiple entries
status: Done
assignee: []
created_date: '2026-05-07 20:53'
updated_date: '2026-05-07 23:35'
labels:
  - code-review-rust
  - err
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/ingestor.rs:74-99` (`MetadataIngestor::load`)

**What**: After ingesting `metadata.json` the loader extracts `workspace_root`:

```rust
let workspace_root: String = conn
    .query_row(
        "SELECT workspace_root FROM metadata_raw ORDER BY rowid LIMIT 1",
        [],
        |row| row.get(0),
    )
    ...
```

The `LIMIT 1` makes this a fail-quiet selector when `metadata_raw` ends up with more than one row. The sister read in `query_metadata_raw` (`lib.rs:175-185`) already learned this lesson and replaced its own `LIMIT 1` with an `ensure!(count == 1, ...)` guard (TASK-0599 / TASK-1034), but the ingestor side still uses the legacy form.

The same line follows it down into `upsert_data_source` and is recorded on the `data_sources` row used as the workspace identity for every subsequent provider lookup. Today the ingest path produces one row per workspace; the moment a future schema (multi-target metadata, schema-version row, partial re-ingest without truncate) inserts a second row, the loader silently writes a `data_sources.workspace_root` keyed off whichever rowid happens to come first, mis-routing every later lookup.

**Why it matters**: ERR-1 — the ingestor enforces an invariant only at the `count(*)` step in `query_metadata_raw`, not here. Loader and reader must agree on the same singleton constraint, otherwise a load that succeeds today silently writes the wrong identity tomorrow with zero log evidence.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 MetadataIngestor::load asserts metadata_raw has exactly one row before reading workspace_root (mirroring query_metadata_raw's ensure!)
- [ ] #2 Regression test: load fails with a clear error when metadata_raw has 0 or >1 rows
<!-- AC:END -->
