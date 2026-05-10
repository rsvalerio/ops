---
id: TASK-0669
title: 'PERF-2: query_project_languages double-scans tokei_files per language row'
status: Done
assignee:
  - TASK-0738
created_date: '2026-04-30 05:14'
updated_date: '2026-04-30 18:30'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/query/loc.rs:73-84`

**What**: The aggregation SQL computes `SUM(code) * 100.0 / NULLIF((SELECT SUM(code) FROM tokei_files), 0)` and `COUNT(*) * 100.0 / NULLIF((SELECT COUNT(*) FROM tokei_files), 0)` — two correlated full-table scans per row of the GROUP BY result.

**Why it matters**: For repos with many languages this is O(L · N) where it could be O(N) using a CTE for the totals. Tokei files can be tens of thousands of rows on monorepos.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Hoist the totals into a CTE (WITH totals AS (SELECT SUM(code) AS tloc, COUNT(*) AS tfiles FROM tokei_files) SELECT … FROM tokei_files, totals GROUP BY language)
- [ ] #2 Bench against a 50k-row tokei_files fixture to confirm a measurable win, or document the rejection
<!-- AC:END -->
