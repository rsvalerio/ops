---
id: TASK-0671
title: >-
  ARCH-6: enrich_from_db gate runs queries even when corresponding identity
  field is populated
status: Done
assignee:
  - TASK-0738
created_date: '2026-04-30 05:14'
updated_date: '2026-04-30 18:33'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/lib.rs:99-105`

**What**: The `enrich_from_db` gate `if identity.loc.is_none() || identity.dependency_count.is_none() || identity.coverage_percent.is_none() || identity.languages.is_empty()` runs even when only a subset of fields is missing; inside `enrich_from_db` the queries run unconditionally, contradicting the gate's intent (it only short-circuits the *call*, not which queries run inside).

**Why it matters**: Combined with the ERR-1 finding on query_project_loc overwriting Some(N) with Some(0), this means a populated identity.loc triggers a needless DuckDB query that may overwrite it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either guard each query inside enrich_from_db with if identity.<field>.is_none(), or remove the outer || chain in favour of always-call + per-field guards
- [ ] #2 Test that a fully-populated provider identity triggers zero DuckDB queries
<!-- AC:END -->
