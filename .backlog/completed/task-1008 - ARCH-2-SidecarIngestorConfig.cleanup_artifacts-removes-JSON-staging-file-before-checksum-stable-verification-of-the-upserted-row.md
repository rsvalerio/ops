---
id: TASK-1008
title: >-
  ARCH-2: SidecarIngestorConfig.cleanup_artifacts removes JSON staging file
  before checksum-stable verification of the upserted row
status: Done
assignee: []
created_date: '2026-05-04 22:05'
updated_date: '2026-05-05 01:14'
labels:
  - code-review-rust
  - ARCH
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/ingestor.rs:143-175` (`load_with_sidecar`) plus `extensions/duckdb/src/ingestor.rs:218-236` (`persist_record`)

**What**: `load_with_sidecar` pipeline step ordering is:

1. create tables + count records (under DB lock).
2. compute checksum of the staged JSON via `crate::sql::checksum_file(json_path)`.
3. `upsert_data_source` writes the (source_name, workspace_root, source_path, record_count, checksum) row.
4. `cleanup_artifacts` removes the staged JSON file.
5. removes the workspace sidecar.

If step 3's `upsert_data_source` succeeds but the host crashes before step 4 / 5 runs (a `kill -9` mid-`fs::remove_file`, a power loss, an OOM kill from another tenant), the next run sees:

- DuckDB row says checksum X for path P.
- Path P still exists on disk with content Y (because cleanup didn't run yet).

…and the next ingest cycle has no way to tell that the on-disk content is *still* the just-loaded staged data rather than a newer scrape. Combined with the `table_has_data` short-circuit in `provide_via_ingestor` (sql/ingest.rs:327), the next caller skips collect entirely because the table is non-empty — so the leftover JSON sits forever as garbage, and any future audit comparing on-disk staged content against the recorded checksum gets a false-positive "unchanged" because the bytes match the stored hash.

**Why it matters**:
- The contract documented at lines 109-148 of ingestor.rs claims "On error, this function is **idempotent on retry**" and lists the cleanup step (#7-#8) as best-effort. That's true for the failure-during-load case but obscures the failure-after-success case where the row is durable but the stage isn't cleaned. The `data_sources` row's `source_path` then references a file the system fully expects to be deleted — operators looking at `target/ops/data.duckdb.ingest/` see "leftover files" and can't tell if they're crash debris or a live ingest mid-flight.
- The structural fix is to make the JSON path purely temporary: write to `<json_path>.tmp` (TASK-0911 already adopted this), upsert with the checksum of the tmp file, and then *rename* the tmp file out of the way (`<json_path>.done` or just unlink) under the same lock as the upsert. Failing that, record the cleanup state in the data_sources row itself (a `cleanup_pending BOOL` column) so a subsequent run can drive the deletion idempotently.
- Lower-cost mitigation: have `provide_via_ingestor`'s `table_has_data == true` early-return also unlink any leftover `<json_filename>` and `_workspace.txt` sidecar in the data dir, so the post-crash cleanup eventually happens on the next user-driven invocation instead of accumulating forever.

**Note**: this is a maintainability / forensic-clarity finding rather than a correctness bug — DuckDB's view of the world is consistent. But the documentation contract and the operational reality drift exactly enough to make a `target/ops/data.duckdb.ingest/` audit confusing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Decision documented (in the rustdoc on load_with_sidecar) on whether leftover JSON after a successful upsert is expected, and what an operator should do about it.
- [ ] #2 Either: (a) provide_via_ingestor's table_has_data short-circuit unlinks any leftover JSON / sidecar before returning, OR (b) persist_record renames the JSON to a .done suffix (or unlinks under the same lock as the upsert).
- [ ] #3 Test pins the 'kill between upsert and cleanup' scenario by simulating the failure and asserting the next run leaves no leftover staged JSON.
<!-- AC:END -->
