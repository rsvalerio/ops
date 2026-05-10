---
id: TASK-0648
title: >-
  ARCH-6: DataIngestor::checksum trait method and skip-if-unchanged docs are
  unimplemented dead code
status: Done
assignee:
  - TASK-0738
created_date: '2026-04-30 04:53'
updated_date: '2026-04-30 18:27'
labels:
  - code-review-rust
  - arch
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/ingestor.rs:301` (trait method) and `extensions/duckdb/src/sql/ingest.rs:215-240` (provide_via_ingestor)

**What**: The `DataIngestor` trait documents a "skip-if-unchanged" lifecycle in which `refresh_metadata` calls `checksum()` first and only re-runs `collect()` + `load()` when the checksum changes (see doc-comment at ingestor.rs:252-258). The actual orchestrator is `provide_via_ingestor` (sql/ingest.rs:215), which never calls `ingestor.checksum()`. It only branches on `table_has_data(table)`. Production call sites for `DataIngestor::checksum` are zero (see `grep -rn ".checksum(" extensions/`). The `SidecarIngestorConfig::checksum` helper at `extensions/duckdb/src/ingestor.rs:240` is also production-dead.

Additionally, `cleanup_artifacts` (ingestor.rs:221) deletes the staged JSON after a successful load, so even if `checksum()` were called later it would now fail with `DbError::Io(NotFound)` because `crate::sql::checksum_file` opens the deleted file.

**Why it matters**: The trait is publicly re-exported (`extensions/duckdb/src/lib.rs:17`) so out-of-crate ingestors see a documented contract that the workspace itself has not implemented. New ingestor authors copy the doc and implement `checksum()`, then are surprised it never runs. Dead code in a public trait is also a maintenance trap — refactors keep paying complexity for a code path that has no callers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either wire skip-if-unchanged into provide_via_ingestor (call ingestor.checksum() before re-collecting and compare against schema::get_source_checksum) or remove the unused trait method + doc lifecycle, retaining only the contract that is actually used
- [ ] #2 If checksum is retained, ensure cleanup_artifacts does not remove the JSON staging file before checksum can be re-read, or relocate the canonical checksum source to the data_sources tracking row
- [ ] #3 Verify no behavior change for current ingestors (tokei is the only impl)
<!-- AC:END -->
