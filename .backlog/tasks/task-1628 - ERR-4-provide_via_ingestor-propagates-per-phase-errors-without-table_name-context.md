---
id: TASK-1628
title: >-
  ERR-4: provide_via_ingestor propagates per-phase errors without table_name
  context
status: Done
assignee:
  - TASK-1640
created_date: '2026-05-22 07:12'
updated_date: '2026-05-22 13:43'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest/orchestrator.rs:99-111`

**What**: Inside `provide_via_ingestor`, the per-phase calls bubble errors without wrapping in a context that names the failing phase or the table:

```
if ctx.refresh { drop_table_if_exists(db, table_name)?; }
if !table_has_data(db, table_name)? {
    let data_dir = data_dir_for_db(db.path());
    create_ingest_dir(&data_dir).map_err(DbError::Io)?;
    ingestor.collect(ctx, &data_dir)?;
    crate::init_schema(db)?;
    let _load_result = ingestor.load(&data_dir, db)?;
}
query_fn(db)
```

When two ingestors fail at the same syscall, the only operator-useful signal is the table name. The current error chain often surfaces a raw `DbError::Io` or `anyhow::Error` with no phase label, forcing operators to grep the codebase to localise the failure.

**Why it matters**: ERR-4 calls out missing `.with_context` on `?` propagation. With table_name already in scope as `&'static str`, attaching it costs one short closure per `?` and turns "ingest collect failed" into "provide_via_ingestor(crate_dependencies): ingest collect failed".
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each ? inside the no-data block carries .with_context naming both the phase (drop/probe/create-dir/collect/init-schema/load) and the table_name
- [ ] #2 A test asserts that a failing collect surfaces table_name in the error chain
- [ ] #3 Existing tests pass; no double-wrapping of an already-contextualised DbError
<!-- AC:END -->
