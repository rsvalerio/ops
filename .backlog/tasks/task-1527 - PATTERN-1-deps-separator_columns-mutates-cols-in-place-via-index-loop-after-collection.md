---
id: TASK-1527
title: >-
  PATTERN-1: deps separator_columns mutates cols in-place via index loop after
  collection
status: Done
assignee:
  - TASK-1645
created_date: '2026-05-19 07:33'
updated_date: '2026-05-25 17:45'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse/upgrade.rs:301-323`

**What**: After collecting `(start, end)` byte ranges, the function loops `for idx in 0..cols.len().saturating_sub(1) { cols[idx].1 = cols[idx + 1].0; }` to stretch each column to the next column's start. The indexed mutation is harder to read and harder to verify against bounds than a windows/zip iterator pass.

**Why it matters**: A future contributor reading this won't immediately see whether the off-by-one is intentional; rewriting with iterator pairs makes the column-stretching invariant obvious and removes the indexed-mutation hazard.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace the index loop with an iterator pass (e.g., build stretched list in one pass via windows/zip)
- [ ] #2 Same output for unit-tested inputs (parse_upgrade_table_basic)
- [ ] #3 Behaviour pinned by an added/extended test on the column stretch semantics if not already covered
<!-- AC:END -->
