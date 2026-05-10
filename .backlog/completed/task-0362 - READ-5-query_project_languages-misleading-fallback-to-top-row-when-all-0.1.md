---
id: TASK-0362
title: 'READ-5: query_project_languages misleading fallback to top row when all <0.1%'
status: Done
assignee:
  - TASK-0420
created_date: '2026-04-26 09:36'
updated_date: '2026-04-27 11:35'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/query/loc.rs:99`

**What**: When every language contributes <0.1% of LOC, the function returns the single top entry rather than the empty filtered result. The doc string says "languages contributing under 0.1% are omitted" — fallback contradicts the contract.

**Why it matters**: Hides whether the project has only tiny entries vs. real data, breaking caller assumptions and producing confusing UI rows.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either remove the fallback (return empty) or update the doc and tests to spell out the exact semantics
- [ ] #2 Test added covering all entries <0.1% returning the documented behavior
<!-- AC:END -->
