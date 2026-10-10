---
id: TASK-2586
title: 'READ-13: tokei_languages_view_sql doc narrates task history and what it replaced instead of the end state'
status: To Do
assignee: []
created_date: '2026-10-10 15:46'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2626'
modified_files:
  - extensions/tokei/src/views.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/tokei/src/views.rs:tokei_languages_view_sql'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/views.rs:31-41` (plus test-module tags at `:56-57`, `:88-89`)

**What**: The doc comment on `tokei_languages_view_sql` documents the backlog rather than the API: it opens with rule/task tags (`SEC-12 (TASK-0593) / ERR-5 (TASK-1003)`, `SEC-12 / TASK-1864`) and narrates the migration ("replaces the runtime `quoted_ident` Result", "a runtime-derived `String` can no longer reach `load_with_sidecar`"). The test-module comments at :56-57 and :88 carry the same `SEC-12:` tags.

**Why it matters**: READ-13 — docs describe the end state, not the journey. Task IDs and "what this replaced" are process artifacts: meaningless to a reader using the API, stale on the next change while looking authoritative. The enduring facts (identifiers are const-validated `TableName` newtypes; the view body is a `&'static str` gated behind `CreateViewSql`) survive as present-tense statements. lib.rs was already cleaned by TASK-2415; views.rs was not in its scope.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Doc on tokei_languages_view_sql states the current invariants (const-validated identifiers, gated view SQL) without rule/task IDs or references to what the code replaced
- [ ] #2 Test comments at views.rs:56-57 and :88 keep their behavioral rationale without the SEC-12 task-tag prefix
<!-- AC:END -->
