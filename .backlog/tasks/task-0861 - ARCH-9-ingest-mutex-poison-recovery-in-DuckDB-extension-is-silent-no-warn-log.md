---
id: TASK-0861
title: >-
  ARCH-9: ingest mutex poison recovery in DuckDB extension is silent (no warn
  log)
status: Triage
assignee: []
created_date: '2026-05-02 09:19'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:282-285` and `extensions/duckdb/src/connection.rs:118-124`

**What**: Both sites use unwrap_or_else(|poisoned| poisoned.into_inner()) and the rationale is documented. But the recovery is silent: no tracing::warn! fires when poison is observed, so an operator looking at production logs cannot distinguish "ingest never panicked" from "ingest panicked once and we recovered".

**Why it matters**: Poison recovery is the right call for a ()-guarded mutex; silent recovery means a transient panic in collect/load (real bug) leaves no breadcrumb.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 On into_inner recovery, emit tracing::warn!(table = %table_name, per-table ingest mutex was poisoned by a prior panic; recovered)
- [ ] #2 Add a test (extending panic_in_collect_does_not_brick_subsequent_ingest) that captures the warn event
- [ ] #3 Document the operator-visible signal in the rustdoc on ingest_mutex_for
<!-- AC:END -->
