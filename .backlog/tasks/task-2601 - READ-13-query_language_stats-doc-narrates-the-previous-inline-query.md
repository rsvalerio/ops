---
id: TASK-2601
title: 'READ-13: query_language_stats doc narrates the previous inline query'
status: Done
assignee: []
created_date: '2026-10-10 20:49'
updated_date: '2026-10-10 21:53'
labels:
  - code-review-rust
  - read
dependencies: []
parent_task_id: 'TASK-2623'
modified_files:
  - extensions/about/src/code.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/about/src/code.rs:query_language_stats'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/code.rs:15`

**What**: The doc comment on `query_language_stats` justifies the delegation by narrating the superseded implementation: "**The previous inline aggregate query** lacked percentages, used a different `LanguageStat` shape, and could drift from the canonical query without anyone noticing."

**Why it matters**: READ-13: documentation describes the end state, not the journey. A reader using the API needs to know it delegates to the shared helper so the `about code` page matches the canonical query — not what the code looked like before. The "previous" framing goes stale on the next change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Doc states that the helper delegates to the shared canonical query, with no reference to a previous implementation

<!-- AC:END -->
