---
id: TASK-2443
title: 'READ-13: go_mod module docs narrate TASK history instead of the end state'
status: To Do
assignee: []
created_date: '2026-10-10 15:23'
updated_date: '2026-10-10 21:12'
labels:
  - code-review
  - read
dependencies: []
parent_task_id: 'TASK-2621'
modified_files:
  - extensions-go/about/src/go_mod.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-go/about/src/go_mod.rs:go_mod'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/go_mod.rs:6`

**What**: The `go_mod` module doc explains replace-directive handling and then narrates how it got that way: "(TASK-2178 dropped replaces from `module_count`; TASK-2254 removed the then-reader-less target collection)". Task IDs and removed-code history are process artifacts; the doc should state only what the parser does today.

**Why it matters**: Documentation describes the end state, not the journey that produced it (READ-13). Task narration is meaningless to a reader using the code and goes stale on the next change while looking authoritative. The project has been stripping exactly this shape (commits "strip task tags and history narration from docs and tests", "remove task provenance tags from module docs").
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Module doc describes current parser behavior only, with no TASK-XXXX references and no narration of removed code
- [ ] #2 grep -r 'TASK-' extensions-go/about/src returns no hits
<!-- AC:END -->
