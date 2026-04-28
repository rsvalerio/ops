---
id: TASK-0471
title: >-
  PATTERN-3: RustUnitsProvider sorted_members + read_crate_metadata clone every
  member path
status: To Do
assignee:
  - TASK-0533
created_date: '2026-04-28 05:47'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/units.rs:32-73`

**What**: members.clone() on lines 32-33, then sort, then .iter().map(...) allocates member.clone() into each ProjectUnit.path. read_crate_metadata returns (Option<String>, Option<String>, Option<String>) even though callers only read &str — allocations stack up across hundreds of crates.

**Why it matters**: OWN-8 / PERF-3: cloning to satisfy ownership rather than borrowing. The function already owns the member strings; pass `members` by value or keep slices.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Refactor RustUnitsProvider::provide to consume members directly (no double clone) and avoid the .clone() in the per-unit map closure where unnecessary
- [ ] #2 Document the trade-off (or measure) so the change is not regressed
<!-- AC:END -->
