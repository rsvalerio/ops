---
id: TASK-0230
title: 'ERR-4: run_about_units swallows duckdb/tokei warm-up errors with let _ ='
status: Done
assignee: []
created_date: '2026-04-23 06:34'
updated_date: '2026-04-23 08:57'
labels:
  - rust-code-review
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/units.rs:22`

**What**: Same `let _ = ctx.get_or_provide(...)` pattern for warm-up providers.

**Why it matters**: Enrichment silently degrades with zero signal.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Log warnings on Err
- [x] #2 Only ignore DataProviderError::NotFound
<!-- AC:END -->
