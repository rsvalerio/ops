---
id: TASK-0609
title: >-
  READ-2: parse_upgrade_table local closure named take shadows
  std::iter::Iterator::take
status: Triage
assignee: []
created_date: '2026-04-29 05:20'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse.rs:85`

**What**: Local closure named `take` shadows Iterator::take, making future edits confusing. The closure indexes cols[idx] after validating cols.len() < 5, but note extraction at line 117 uses cols.get(5) — inconsistent indexing. Function does five things at ~90 lines.

**Why it matters**: Readability — shadowed names plus mixed indexing increase cognitive load on parser hot path that has had two recent fixes (TASK-0383/0404). FN-1 candidate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Closure renamed to domain-specific name (e.g. slice_col) so it does not shadow Iterator::take
- [ ] #2 Either both row-field and note use cols.get, or both index directly
<!-- AC:END -->
