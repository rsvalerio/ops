---
id: TASK-2798
title: 'DUP-3: about card enrich_from_db repeats the same query/match/warn block five times'
status: Done
assignee: []
created_date: '2026-11-18 18:46'
updated_date: '2026-11-18 21:36'
labels:
  - code-review-rust
  - DUP
dependencies: []
parent_task_id: 'TASK-2858'
modified_files:
  - extensions/about/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'DUP-3:extensions/about/src/lib.rs:enrich_from_db'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/lib.rs:208-276` (`enrich_from_db`, sqlite build)

**What**: Lines 232-236, 244-248, 251-255, 258-262 and 265-271 each repeat `match query(db) { Ok(v) if <non-zero> => identity.<field> = Some(v), Ok(_) => {}, Err(e) => tracing::warn!(error = ?e, "about: <query> failed") }`, differing only in the query, the non-zero predicate and the target field. The function is 68 lines (over FN-1's 50) largely because of the repetition.

**Why it matters**: DUP-3 flags 3+ repetitions of the same error-mapping shape. A small helper (`fill_if_none(&mut identity.loc, "query_project_loc", || query(db), |v| v > 0)`) would shrink the function, keep the warn wording uniform, and make adding the next enrichment a one-liner.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The repeated query/match/warn shape is expressed once in a helper used by every enrichment
- [x] #2 enrich_from_db is under 50 lines and existing lib.rs tests pass with the sqlite feature on

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.22 (code-review-rust run, 2026-11-18).

query_kept helper + fill_rust_regions split; enrich_from_db is now 35 lines.

<!-- SECTION:NOTES:END -->
