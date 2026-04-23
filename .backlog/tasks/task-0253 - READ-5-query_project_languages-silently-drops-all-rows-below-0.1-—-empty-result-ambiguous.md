---
id: TASK-0253
title: >-
  READ-5: query_project_languages silently drops all rows below 0.1% — empty
  result ambiguous
status: To Do
assignee: []
created_date: '2026-04-23 06:35'
updated_date: '2026-04-23 06:46'
labels:
  - rust-code-review
  - readability
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/query/loc.rs:92`

**What**: `if stat.loc_pct >= 0.1` filter has no fallback; a project whose largest language is 0.05% returns Vec::new().

**Why it matters**: Callers cannot tell "no tokei data" from "all languages tiny".
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Always keep top N regardless of threshold or aggregate remainder into 'other'
- [ ] #2 Test with inputs where max loc_pct < 0.1
<!-- AC:END -->
