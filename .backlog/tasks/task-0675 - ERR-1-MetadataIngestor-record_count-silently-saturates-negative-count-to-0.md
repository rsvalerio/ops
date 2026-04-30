---
id: TASK-0675
title: 'ERR-1: MetadataIngestor record_count silently saturates negative count to 0'
status: Done
assignee:
  - TASK-0738
created_date: '2026-04-30 05:14'
updated_date: '2026-04-30 18:33'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/ingestor.rs:56-61`

**What**: `u64::try_from(c).unwrap_or(0)` converts a negative `i64` count to `0` silently before upserting `data_sources`.

**Why it matters**: Hides DuckDB schema corruption (count(*) cannot be negative under SQL semantics, but a future schema change with a signed column or aggregate could). Matches the family of TASK-0651 (negative coercion) but on a different call site, so listing it for awareness rather than as a duplicate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Treat a negative count as a schema invariant violation: return DbError::query_failed instead of clamping
<!-- AC:END -->
