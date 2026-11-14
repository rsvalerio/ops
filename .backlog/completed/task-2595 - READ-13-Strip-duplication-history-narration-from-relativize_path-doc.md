---
id: TASK-2595
title: 'READ-13: Strip duplication-history narration from relativize_path doc'
status: Done
assignee: []
created_date: '2026-10-10 15:47'
updated_date: '2026-10-10 22:04'
labels:
  - code-review-rust
  - read
dependencies: []
parent_task_id: 'TASK-2625'
modified_files:
  - extensions/sqlite/src/sql/mod.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/sqlite/src/sql/mod.rs:relativize_path'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/sqlite/src/sql/mod.rs:113`

**What**: The `relativize_path` doc (around lines 110-116) recounts that "the tokei and rust-loc extensions each carried a byte-identical copy of this helper; the policy now lives here, once, ..." — a description of how the helper came to be centralized.

**Why it matters**: READ-13: docs describe the end state, not the journey. The lasting contract is what the function does (strip the workspace-root prefix, returning the remainder) and any invariants; which extensions used to duplicate it is changelog material, not API documentation. Survived the TASK-2408 sweep. Fix: keep the behavioural description and invariants, drop the "each carried a byte-identical copy ... now lives here" history.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The relativize_path doc states only current behaviour and invariants, with no account of former duplicates in other extensions

<!-- AC:END -->
