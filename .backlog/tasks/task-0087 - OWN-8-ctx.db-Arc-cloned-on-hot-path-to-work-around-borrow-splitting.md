---
id: TASK-0087
title: 'OWN-8: ctx.db Arc cloned on hot path to work around borrow splitting'
status: To Do
assignee: []
created_date: '2026-04-17 11:33'
updated_date: '2026-04-17 12:07'
labels:
  - rust-codereview
  - own
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/lib.rs:38`

**What**: try_provide_from_db clones ctx.db Arc purely to split the borrow so fallback_fn can take &mut Context.

**Why it matters**: Every call allocates an Arc refcount bump on the happy path; the fallback closure design forces this.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Refactor API so callers decide whether to clone; e.g., return Option<&DuckDb> and let caller dispatch
- [ ] #2 If clone stays, benchmark to confirm refcount cost is negligible and document justification
<!-- AC:END -->
