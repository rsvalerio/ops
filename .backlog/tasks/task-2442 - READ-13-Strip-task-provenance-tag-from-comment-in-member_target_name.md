---
id: TASK-2442
title: 'READ-13: Strip task-provenance tag from comment in member_target_name'
status: Done
assignee: []
created_date: '2026-10-10 15:21'
updated_date: '2026-10-10 22:02'
labels:
  - code-review
  - readability
dependencies: []
parent_task_id: 'TASK-2620'
modified_files:
  - extensions-rust/create-review-tasks/src/provider.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/create-review-tasks/src/provider.rs:member_target_name'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/create-review-tasks/src/provider.rs:119-122`

**What**: The comment inside `member_target_name` opens with `// DUP-1 / TASK-2251:` and self-reports the rules the change followed (`ERR-7: embedded newlines and ANSI escapes cannot forge log records`). This is process narration — a backlog-task tag plus a guideline self-report — not a description of the end state. The enduring rationale it partially carries (the shared helper Debug-formats the untrusted `member` and tags the surface via the `site` field) survives without the tag.

**Why it matters**: READ-13: docs and comments describe the end state, not the journey that produced it. Task-provenance tags are meaningless to a reader using the code, go stale on the next change while looking authoritative, and belong in the PR description or the backlog task itself. This repo's own recent history (commits stripping task tags from theme, about, and sqlite docs) treats this exact pattern as cleanup debt.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The `DUP-1 / TASK-2251:` prefix and rule-ID self-report are removed from the comment in `member_target_name`
- [x] #2 The enduring rationale (shared helper Debug-formats the untrusted member; `site` field tags this surface) is retained without the provenance tag

<!-- AC:END -->
