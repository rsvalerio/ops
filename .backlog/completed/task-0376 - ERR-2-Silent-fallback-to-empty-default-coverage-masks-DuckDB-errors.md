---
id: TASK-0376
title: 'ERR-2: Silent fallback to empty/default coverage masks DuckDB errors'
status: Done
assignee:
  - TASK-0420
created_date: '2026-04-26 09:38'
updated_date: '2026-04-27 11:39'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/coverage_provider.rs:53`

**What**: When query_crate_coverage fails, the code calls .unwrap_or_default() (silently dropping the error). Same pattern in metrics.rs (.ok() on every query) and deps_provider.rs:23 (.unwrap_or_default()). The user sees empty coverage with no signal whether the data is empty or the query failed.

**Why it matters**: A DuckDB schema mismatch or migration bug becomes invisible — coverage simply renders blank.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 At minimum log at tracing::warn! with the error before falling back to default (consistent with the one debug line that exists at coverage_provider.rs:34)
- [ ] #2 Decide policy and apply consistently across all four DuckDB query call sites in metrics.rs, coverage_provider.rs, units.rs, and deps_provider.rs
<!-- AC:END -->
