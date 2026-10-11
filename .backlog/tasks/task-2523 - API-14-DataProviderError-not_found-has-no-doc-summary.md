---
id: TASK-2523
title: 'API-14: DataProviderError::not_found has no doc summary'
status: Done
assignee: []
created_date: '2026-10-10 15:33'
updated_date: '2026-10-10 21:25'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2612'
modified_files:
  - crates/extension/src/error.rs
priority: low
ordinal: 1000
dedup_key: 'API-14:crates/extension/src/error.rs:DataProviderError::not_found'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/error.rs:237`

**What**: `DataProviderError::not_found` is the only public item in the error module with no doc comment. Its two sibling constructors (`computation_failed`, `computation_error`) are documented, and the `NotFound` variant it builds has extensive docs — the constructor itself says nothing.

**Why it matters**: API-14 — the summary sentence on a public item is mandatory. The doc should state when to use it versus matching the variant directly, and that warm-up callers commonly treat `NotFound` as an expected "not part of the active stack" answer (the contract the variant docs already spell out).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 DataProviderError::not_found carries a doc comment whose first paragraph is a ~15-word summary, including the expected-NotFound warm-up idiom

<!-- AC:END -->
