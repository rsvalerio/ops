---
id: TASK-0959
title: >-
  ERR-1: query_per_crate_i64 silently surfaces negative i64 from corrupted
  DuckDB views
status: Triage
assignee: []
created_date: '2026-05-04 21:46'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/query/helpers.rs:319` (query_per_crate_i64)

**What**: `query_per_crate_i64` collects `row.get::<_, i64>(1)` into `HashMap<String, i64>` with no negativity guard. SUM(code)/COUNT(*) cannot legitimately go negative, but a corrupted view or schema bug can produce one. TASK-0506 already added `coerce_count_to_usize` with a tracing::warn for the project-scalar path. Per-crate values reach `extensions/about/src/units.rs::enrich_from_db` as `unit.loc = Some(-123)`, presenting a schema bug as "this crate has -123 lines".

**Why it matters**: Closes the TASK-0506 sweep gap on the per-crate path. Zero happy-path overhead, discoverable when DuckDB shape drifts.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Per-crate i64 emits tracing::warn and clamps to 0 on negative, matching coerce_count_to_usize policy
- [ ] #2 Unit test feeds a synthetic negative row and asserts final map value is 0
<!-- AC:END -->
