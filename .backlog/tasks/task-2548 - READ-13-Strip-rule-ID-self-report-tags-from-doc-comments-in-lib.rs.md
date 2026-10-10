---
id: TASK-2548
title: 'READ-13: Strip rule-ID self-report tags from doc comments in lib.rs'
status: To Do
assignee: []
created_date: '2026-10-10 15:39'
updated_date: '2026-10-10 21:12'
labels:
  - code-review
  - readability
dependencies: []
parent_task_id: 'TASK-2620'
modified_files:
  - extensions/create-review-tasks/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/create-review-tasks/src/lib.rs:<crate docs>'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/create-review-tasks/src/lib.rs:32,119,214,288,343,428,445,453,532` (production docs) and `:1152,1184,1224,1254,1288` (test docs)

**What**: Doc comments across the file open with rule-ID self-report tags — `PATTERN-1:` (line 32, `RunMode`), `ERR-6:` (119, `run_create_review_tasks_with_clock`), `SEC-11 layer 3 (format):` (214, `validate`), `FN-3:` (288, `PlannedSubtask`; 428, `TaskFile`), `SEC-25:` (343, `commit_task_set`; 453, `stage_task_file`), `ERR-13:` (445, `stage_task_file`), `SEC-11:` (532, `report`; test docs 1224, 1254, 1288), `TEST-5:`/`TEST-6:` (test docs 1152, 1184). These annotate which review rules the change followed — process narration, not a description of the end state. Most of the tagged comments carry enduring rationale that survives verbatim once the tag prefix is removed.

**Why it matters**: READ-13: docs describe the end state, not the journey that produced it. Rule-ID self-reports are meaningless to a reader using the API, go stale on the next change while looking authoritative, and belong in the PR description or the review task itself. This repo's own recent history (commits stripping task tags and narration from theme, about, and sqlite docs and tests) treats this exact pattern as cleanup debt; the sibling review of extensions-rust/create-review-tasks filed the same rule (TASK-2442).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Rule-ID tag prefixes (PATTERN-1:, ERR-6:, SEC-11:, SEC-25:, ERR-13:, FN-3:, TEST-5:, TEST-6:) are removed from every listed doc comment in src/lib.rs
- [ ] #2 The enduring rationale each comment carries (why the clock is read first, why create_new is used, why validation sits at the boundary) is retained as plain prose without the tag
<!-- AC:END -->
