---
id: TASK-2593
title: 'READ-13: Strip journey narration from deps.rs docs and log message'
status: To Do
assignee: []
created_date: '2026-10-10 15:47'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - read
dependencies: []
parent_task_id: 'TASK-2625'
modified_files:
  - extensions/sqlite/src/sql/query/deps.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/sqlite/src/sql/query/deps.rs:query_dependency_count'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/sqlite/src/sql/query/deps.rs:11`, `extensions/sqlite/src/sql/query/deps.rs:100`, `extensions/sqlite/src/sql/query/deps.rs:146`

**What**: Three spots narrate how the code used to behave rather than what it does now:
- `deps.rs:11` (query_dependency_count doc): "... used to be silently coerced ... Now we surface ..."
- `deps.rs:100` (query_crate_dep_counts doc): "The previous shape silently mis-attributed ..."
- `deps.rs:146` (tracing::debug! message): "dep_counts now key by manifest_path" — the word "now" is migration narration inside a runtime operator-facing message.

**Why it matters**: READ-13: docs should describe the end state, not the journey. A reader of today's code has never seen the old shape, so each "used to / previous / now" clause forces them to reconstruct a historical diff they don't care about. Fix: state the current contract directly (e.g. "keys are manifest paths, because crate_name can map to multiple workspace members"; for the doc at line 11, describe only the current error-surfacing behaviour). A prior wave (TASK-2408) already removed most of this crate's history narration; these three remain.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No doc comment or log message in deps.rs contains "used to", "previous", or a "now X" contrast with a past behaviour; each describes only the current contract
<!-- AC:END -->
