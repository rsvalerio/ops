---
id: TASK-0933
title: >-
  SEC-25: MetadataIngestor::collect writes metadata.json with std::fs::write
  (non-atomic, regressing TASK-0911 sweep)
status: Done
assignee: []
created_date: '2026-05-02 15:50'
updated_date: '2026-05-02 16:18'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/ingestor.rs:31`

**What**: `MetadataIngestor::collect` persists `cargo metadata` stdout to `data_dir/metadata.json` via `std::fs::write(&path, &output.stdout).map_err(DbError::Io)?;`. This is a non-atomic write — a crash, SIGKILL, OOM, or filesystem error mid-write leaves a truncated/partial JSON file at the canonical path. The next `load()` call then runs DuckDB's `read_json` over the corrupt file and either fails or — worse — silently materialises a wrong workspace metadata row.

The sibling sidecar pipeline already routes through `atomic_write` after TASK-0911 (`fix(duckdb/ingestor): collect_sidecar JSON write via atomic_write` — commit 6102f4c). MetadataIngestor was not swept in that fix and remains the only pre-DuckDB JSON drop site that does not use the project's `atomic_write` helper.

**Why it matters**: An interrupted `cargo metadata` stage produces a poisoned `metadata.json` at the well-known path. Because `collect()` returns `Err` on the underlying IO error, the surrounding orchestrator may not retry until the next `ops about` invocation, by which time the partial file looks identical to a successful one to the load path. Atomic write (write-tmp + fsync + rename) is the documented project-wide remedy and matches the sibling fix shipped 5 commits ago.

<!-- scan confidence: candidates to inspect -->

- Candidate site: `extensions-rust/metadata/src/ingestor.rs:31`
- Sibling already-fixed pattern: `ops_duckdb::SidecarIngestorConfig::collect_sidecar` (per TASK-0911)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 metadata.json write goes through ops_core::atomic_write (or ops_duckdb's equivalent) instead of std::fs::write
- [ ] #2 Test demonstrates that a simulated mid-write failure leaves either no metadata.json or the previous version, never a partial
- [ ] #3 No regression in MetadataIngestor::collect happy path (existing tests still pass)
- [ ] #4 1:check,2:check,3:check
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Switched MetadataIngestor::collect to ops_core::config::atomic_write (sibling temp + fsync + rename), matching the TASK-0911 fix for SidecarIngestorConfig::collect_sidecar. A crash mid-write previously left a torn or zero-byte metadata.json that the subsequent load step would feed to DuckDB read_json_auto, corrupting the database with truncated input. Added regression test metadata_collect_writes_atomically_no_tmp_leftover that calls collect against this crate's manifest and asserts (a) metadata.json exists and (b) no .tmp.* sibling lingers from the atomic_write helper, mirroring the test added in TASK-0911. Existing happy-path tests pass; cargo fmt / clippy / build clean.
<!-- SECTION:NOTES:END -->
