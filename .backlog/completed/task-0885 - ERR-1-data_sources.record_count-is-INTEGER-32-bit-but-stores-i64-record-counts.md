---
id: TASK-0885
title: >-
  ERR-1: data_sources.record_count is INTEGER (32-bit) but stores i64 record
  counts
status: Done
assignee: []
created_date: '2026-05-02 09:37'
updated_date: '2026-05-02 12:01'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/schema.rs:17`

**What**: The `data_sources` table is created with `record_count INTEGER NOT NULL DEFAULT 0`, and `upsert_data_source` binds an `i64` (`record_count_i64`) into that column. DuckDB `INTEGER` is 32-bit signed (i32 range ±2,147,483,647). Counts that fit in i64 but exceed i32::MAX (e.g. a tokei ingest of a very large monorepo where files+code rows exceed ~2.1B is rare, but coverage_files row counts on a large monorepo can plausibly reach single-digit millions and the column also stores per-source totals that could include sums) will either be rejected by DuckDB at bind time or silently truncated, depending on driver behaviour. Either way the stored value is not the intended `i64`.

**Why it matters**: Schema/storage type mismatch is a silent correctness bug. The Rust side computes `i64::try_from(meta.record_count)` and surfaces `RecordCountOverflow` on `u64 > i64::MAX`, but never checks the i32 ceiling that the actual column enforces. A future ingest that crosses 2^31 will either fail with an opaque DuckDB bind error or silently store a wrong count, breaking the change-detection checksum logic that downstream `provide_via_ingestor` relies on. Use `BIGINT` to match the i64 the Rust API already promises.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 data_sources.record_count column type widened to BIGINT (or schema migration introduced)
- [ ] #2 regression test confirms a count > i32::MAX round-trips through upsert_data_source / get_source_checksum
<!-- AC:END -->
