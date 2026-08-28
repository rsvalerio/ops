---
id: TASK-1891
title: >-
  ERR-1: multi-row metadata_raw passes ingest with a warn but hard-fails every
  read, bricking the DuckDB file
status: To Do
assignee:
  - TASK-1999
created_date: '2026-08-27 15:35'
updated_date: '2026-08-28 14:13'
labels:
  - code-review-rust
  - idioms-correctness
dependencies: []
modified_files:
  - extensions-rust/metadata/src/ingestor.rs
  - extensions-rust/metadata/src/lib.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/ingestor.rs:52` and `extensions-rust/metadata/src/lib.rs:259`

**What**: The two halves of the crate disagree about the `metadata_raw` singleton invariant.

`MetadataIngestor::load` treats >1 row as a *warning* and returns success:

```rust
let record_count = query_record_count(&conn)?;
if record_count > 1 {
    tracing::warn!(rows = record_count, "metadata_raw has multiple workspace_root rows; using first");
}
```

`query_metadata_raw_with_cap` — the only reader — treats the same state as a hard error:

```rust
anyhow::ensure!(count == 1, "metadata_raw must contain exactly one row, found {count}");
```

`provide_via_ingestor` (ops-duckdb orchestrator) runs `collect` -> `load` -> `query_fn`, so the moment `load` accepts a 2-row table, the very next call in the same request fails. Worse, the table now `table_has_data()`, so every subsequent `ops about` run skips re-ingest, goes straight to the reader and fails again. The DuckDB file is left permanently unreadable for this provider until someone finds `--refresh`.

The existing test `metadata_load_warns_when_metadata_raw_has_multiple_rows` (ingestor.rs:293) *pins the broken behaviour*: it asserts `result.is_ok()` for exactly the two-row fixture that `query_metadata_raw` then rejects. No test drives `load` followed by `query_metadata_raw` on the same DB, which is why the contradiction survived.

**Why it matters**: silent data-state corruption with a sticky failure mode. `load` is the layer that can still fix the problem (truncate, re-collect, or fail before committing); by deferring the decision to the reader the code guarantees the worst outcome — the bad state is committed and then rejected forever. Either `load` must enforce the same `count == 1` invariant (fail fast, leave the table absent so the next run re-ingests) or the reader must tolerate multiple rows the way `extract_workspace_root` already does with `ORDER BY rowid LIMIT 1`. Both halves cannot be right.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 MetadataIngestor::load and query_metadata_raw agree on the metadata_raw row-count invariant: either both reject count != 1, or both tolerate it with the same first-row policy
- [ ] #2 When load rejects a multi-row ingest, metadata_raw is left in a state that causes the next run to re-ingest rather than replay the same failure (no sticky unreadable DB)
- [ ] #3 A test drives load() and then query_metadata_raw() against the same DuckDb instance with a two-row fixture and asserts the combined outcome, not just load()'s return
- [ ] #4 metadata_load_warns_when_metadata_raw_has_multiple_rows is updated to match the chosen contract instead of asserting is_ok() on a state the reader rejects
<!-- AC:END -->
