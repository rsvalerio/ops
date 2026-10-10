---
id: TASK-2549
title: 'READ-13: Strip rule-ID and task-provenance tags from doc comments in backlog.rs'
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
  - extensions/create-review-tasks/src/backlog.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/create-review-tasks/src/backlog.rs:<module docs>'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/create-review-tasks/src/backlog.rs:28,43,125,184` (production docs) and `:575,608,624,647,658,675` (test docs)

**What**: Doc comments across the file open with rule-ID self-report tags — `READ-11:` (line 28, `TASK_PREFIX`/`ID_WIDTH`), `ERR-13:` (43, `require_backlog_tasks_dir`), `FN-3:` (125, `MainTaskClaim`), `DUP-2:` (184, `render_task_file`) — and test doc comments carry both rule tags (`READ-6:` at 575, `TEST-8 boundary:` at 647, `SEC-11 boundary:`/`SEC-11:` at 658 and 675) and task-provenance tags (`TASK-2435 AC #1:` at 608 and 624). These annotate which review rules or backlog tasks produced the change — process narration, not a description of the end state. The substantive rationale in each comment (why the constants are pinned, why the shared renderer is used) survives verbatim once the tag prefix is removed.

**Why it matters**: READ-13: docs describe the end state, not the journey that produced it. Rule-ID self-reports and TASK-xxxx provenance tags are meaningless to a reader using the code, go stale on the next change while looking authoritative, and belong in the PR description or the review task itself. This repo's own recent history (commits stripping task tags and narration from theme, about, and sqlite docs and tests) treats this exact pattern as cleanup debt; the sibling review of extensions-rust/create-review-tasks filed the same rule (TASK-2442).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Rule-ID tag prefixes (READ-11:, ERR-13:, FN-3:, DUP-2:, READ-6:, TEST-8:, SEC-11:) and TASK-2435 provenance tags are removed from every listed doc comment in src/backlog.rs
- [ ] #2 The enduring rationale each comment carries (why the id width is pinned, why the shared ops_backlog renderer is used, what the boundary tests pin) is retained as plain prose without the tags
<!-- AC:END -->
