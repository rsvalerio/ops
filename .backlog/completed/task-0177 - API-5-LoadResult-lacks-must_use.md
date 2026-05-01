---
id: TASK-0177
title: 'API-5: LoadResult lacks must_use'
status: Done
assignee: []
created_date: '2026-04-22 21:25'
updated_date: '2026-04-23 14:59'
labels:
  - rust-code-review
  - API
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/ingestor.rs:11-14`

**What**: LoadResult returned by DataIngestor::load carries record_count the caller almost always wants to observe. It is not marked must_use, so silently discarding it compiles without warning.

**Why it matters**: API-5 — must_use on result-like types that carry information beyond success. Prevents silent loss of record counts in pipelines.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 LoadResult has must_use attribute with a rationale
<!-- AC:END -->
