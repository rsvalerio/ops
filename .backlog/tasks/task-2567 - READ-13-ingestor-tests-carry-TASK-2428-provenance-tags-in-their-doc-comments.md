---
id: TASK-2567
title: 'READ-13: ingestor tests carry TASK-2428 provenance tags in their doc comments'
status: Done
assignee: []
created_date: '2026-10-10 15:42'
updated_date: '2026-10-10 21:49'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2619'
modified_files:
  - extensions-rust/metadata/src/ingestor.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/metadata/src/ingestor.rs:tests'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/ingestor.rs:403-407`; `extensions-rust/metadata/src/ingestor.rs:443-447`

**What**: Two test doc comments open with task provenance tags: "TASK-2428 AC #1: `io_at` must keep the original `io::Error` reachable through `Error::source()` ..." and "TASK-2428 AC #2: a staged-payload read failure must name the staged entry. ...".

**Why it matters**: READ-13 — task IDs and acceptance-criterion numbering are process artifacts: they point at a backlog entry that a reader of the code has no context for, and they go stale when the task tracker is renumbered or archived while looking authoritative. The repository has already stripped exactly this pattern from sibling crates (commits "docs(about): remove task provenance tags from module docs" and "docs(theme): strip task tags and history narration from docs and tests"). Keep the behavioral statements (what the error chain must preserve, what the read failure must name); drop the TASK/AC framing.

<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Both test doc comments state the pinned behavior without the TASK-2428 AC #N prefix

<!-- AC:END -->
