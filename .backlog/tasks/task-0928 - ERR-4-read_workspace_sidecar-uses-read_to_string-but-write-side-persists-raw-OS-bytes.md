---
id: TASK-0928
title: >-
  ERR-4: read_workspace_sidecar uses read_to_string but write side persists raw
  OS bytes
status: Done
assignee: []
created_date: '2026-05-02 15:32'
updated_date: '2026-05-02 16:22'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:204`

**What**: `write_workspace_sidecar` writes `working_directory.as_os_str().as_encoded_bytes()` (raw OS bytes, intentionally non-UTF-8-preserving per the inline READ-5 comment), but `read_workspace_sidecar` calls `std::fs::read_to_string` which fails with `ErrorKind::InvalidData` on any non-UTF-8 byte. The sister test `workspace_sidecar_round_trips_non_utf8_path` only checks the raw on-disk bytes via `std::fs::read` — it never exercises the read helper, so the asymmetry has been invisible. A workspace with non-UTF-8 path bytes (legal on Linux/macOS) writes successfully then fails on the next ingest with an opaque "stream did not contain valid UTF-8" error originating from `load_with_sidecar`'s sidecar read.

**Why it matters**: The write side was deliberately upgraded to preserve non-UTF-8 paths; the read side silently undermines that contract. On the affected workspace, every `ops` data ingest fails until the user manually deletes the sidecar, with no error message pointing at the cause. Fix is small and the contract is already half-implemented.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 read_workspace_sidecar uses std::fs::read + OsStr::from_encoded_bytes_unchecked (or returns Vec<u8> / OsString) so non-UTF-8 workspace_root values round-trip identically to what write_workspace_sidecar persisted.
- [ ] #2 load_with_sidecar (and any other caller) accepts the new return type without lossy conversion before reaching upsert_data_source, which already handles the NonUtf8Path case for the staged JSON path.
- [ ] #3 New regression test calls write_workspace_sidecar then read_workspace_sidecar (the helper, not raw fs::read) with a non-UTF-8 working_directory and asserts byte-exact equality.
<!-- AC:END -->
