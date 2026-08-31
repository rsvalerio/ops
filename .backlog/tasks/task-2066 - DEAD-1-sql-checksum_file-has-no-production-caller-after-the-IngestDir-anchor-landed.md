---
id: TASK-2066
title: >-
  DEAD-1: sql::checksum_file has no production caller after the IngestDir anchor
  landed
status: Done
assignee: []
created_date: '2026-08-29 18:09'
updated_date: '2026-08-31 17:38'
labels:
  - code-review-rust
  - dead-code
dependencies: []
modified_files:
  - extensions/duckdb/src/sql/ingest/dir.rs
  - extensions/duckdb/src/sql/mod.rs
  - extensions/duckdb/src/sql/ingest/mod.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest/dir.rs`

**What**: TASK-2054 moved the ingest pipeline's only two checksum call sites
(`SidecarIngestorConfig::persist_record` and `MetadataIngestor::load`) onto the
anchored `IngestDir::checksum`, which opens the staged file through the verified
directory descriptor. The path-based `pub fn checksum_file(path: &Path)` it
replaced is still exported from `ops_duckdb::sql` but now has zero production
callers — only this module's own tests and one cross-check assertion in the
TASK-2054 swap test use it.

**Why it matters**: a public helper with no production caller is a live
invitation to re-introduce the by-path resolution the anchor exists to remove: a
future ingestor reaching for `checksum_file(&dir.entry_path(name))` gets exactly
the pre-TASK-2054 behaviour and nothing flags it. Either delete it (and rebase
the remaining tests onto `IngestDir::checksum` plus the private
`checksum_reader`), or narrow it to `pub(crate)` / `#[cfg(test)]` if the
streaming implementation is worth keeping as a test oracle.

Not removed inside TASK-2054's wave because it is public API of `ops_duckdb`
rather than crate-private, so the removal is a surface change rather than the
mechanical dead-code sweep the wave protocol allows in-flight.

**Origin**: discovered during TASK-2063 while fixing TASK-2054.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 sql::checksum_file is either removed or narrowed to a non-public/test-only surface, and no production code path computes an ingest checksum from a re-resolved path
<!-- AC:END -->
