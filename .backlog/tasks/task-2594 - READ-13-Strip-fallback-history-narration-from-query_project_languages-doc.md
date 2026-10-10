---
id: TASK-2594
title: 'READ-13: Strip fallback-history narration from query_project_languages doc'
status: Done
assignee: []
created_date: '2026-10-10 15:47'
updated_date: '2026-10-10 21:57'
labels:
  - code-review-rust
  - read
dependencies: []
parent_task_id: 'TASK-2625'
modified_files:
  - extensions/sqlite/src/sql/query/loc.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/sqlite/src/sql/query/loc.rs:query_project_languages'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/sqlite/src/sql/query/loc.rs:74`

**What**: The `query_project_languages` doc (around lines 72-78) explains: "Previously this function fell back to the top entry when the filtered ... The empty return is now the only signal ..." — a description of a past behaviour contrasted with the current one.

**Why it matters**: READ-13: docs describe the end state, not the journey. The durable fact is simply "returns an empty map when no language rows survive the filter; callers treat empty as the not-collected signal". The "Previously / fell back" framing asks the reader to hold a history they never experienced. Leftover from before the TASK-2408 narration sweep. Fix: delete the historical sentence, keep (or tighten) the statement of the empty-return contract.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The query_project_languages doc describes only the current empty-return contract with no "previously"/past-behaviour sentence

<!-- AC:END -->
