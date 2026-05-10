---
id: TASK-1148
title: >-
  ERR-1: about::lib::enrich_from_db acquires db.lock five times producing
  inconsistent identity card
status: Done
assignee:
  - TASK-1268
created_date: '2026-05-08 07:42'
updated_date: '2026-05-09 17:31'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/lib.rs:152-194`

**What**: Each of the five query_* helpers in enrich_from_db acquires db.lock() independently. A concurrent ingest running between samples can produce an identity card whose loc / file_count / dependency_count / coverage_percent / languages describe different snapshots. The sister units::enrich_from_db documents this explicitly (ERR-1 / TASK-0431); the about-card variant ships the same shape with no doc and no contract.

**Why it matters**: Operator-visible inconsistency on the about card with no documented policy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Add the same TASK-0431 doc comment to lib.rs::enrich_from_db cross-referencing the units variant
- [ ] #2 Or refactor the helpers to take &duckdb::Connection so a single db.lock()? guard scopes all five queries
<!-- AC:END -->
