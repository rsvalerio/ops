---
id: TASK-0475
title: >-
  DUP-1: DuckDB query-with-warn-fallback boilerplate duplicated 7+ times across
  about providers
status: Done
assignee:
  - TASK-0534
created_date: '2026-04-28 05:47'
updated_date: '2026-04-28 18:54'
labels:
  - code-review-rust
  - DUP
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/{deps_provider.rs:26-35, coverage_provider.rs:36-65, units.rs:38-50, identity/metrics.rs:32-104}`

**What**: The same shape repeats: `match query_X(db) { Ok(v) => v, Err(e) => { tracing::warn!(query=..., "duckdb query failed; ... will be ...: {e:#}"); fallback } }`. Counted at least 7 occurrences across the four files.

**Why it matters**: DUP-1 — making future changes (switching tracing field shape, adding span context, mapping to typed degraded values) a 7-place edit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add an ops_duckdb helper such as query_or_warn(db, label, fallback, query_fn) (or a macro) and migrate the call sites
- [ ] #2 All seven sites read identically after migration; fallback values preserved
<!-- AC:END -->
