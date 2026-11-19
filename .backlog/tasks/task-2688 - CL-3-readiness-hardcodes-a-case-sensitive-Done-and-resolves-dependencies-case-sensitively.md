---
id: TASK-2688
title: 'CL-3: readiness hardcodes a case-sensitive "Done" and resolves dependencies case-sensitively'
status: Done
assignee: []
created_date: '2026-11-18 18:27'
updated_date: '2026-11-18 21:41'
labels:
  - code-review-rust
  - CL
dependencies: []
parent_task_id: 'TASK-2838'
modified_files:
  - crates/backlog/src/render.rs
  - crates/backlog/src/cmd/view.rs
priority: medium
ordinal: 1000
dedup_key: 'CL-3:crates/backlog/src/render.rs:readiness_of'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/render.rs:100-115` (`readiness_of`), `crates/backlog/src/render.rs:538-543` (`list_json` lookup), `crates/backlog/src/cmd/view.rs:38-43` (`run_view` lookup), `crates/backlog/src/render.rs:138,355` (Done marks)

**What**: `readiness_of` treats a dependency as finished only when its status is exactly `"Done"` (`status != "Done"`), and a task as finished only when its own status is exactly `"Done"`. Everywhere else the crate takes the terminal status from config (`cleanup::terminal_status`, last configured column) and compares case-insensitively (`cleanup.rs:133`, `about.rs:112`, `create.rs:221`). A backlog whose terminal column is not literally `Done` — or a dependency whose status was written `done` — reports every dependent as blocked forever.

Separately, the id → status lookups handed to `readiness_of` in `run_view` and `list_json` compare ids with `==`, while `about.rs:186-195` explicitly lowercases both sides and documents why: "a dependency written `task-0003` would miss live `TASK-0003`, count as missing — non-blocking — and flip a blocked task to ready". `task view` and `task list --json` still carry that bug, so `about backlog` and `task list`/`view` disagree on readiness for the same tree.

**Why it matters**: `isReady`/`blockingDependencies` drive which tasks agents pick up; wrong readiness either stalls work or lets blocked tasks start.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 readiness_of takes the configured terminal status and compares statuses case-insensitively, consistent with cleanup/about
- [x] #2 view and list JSON resolve dependency ids case-insensitively (one shared lookup builder), matching about.rs readiness_counts
- [x] #3 Tests cover a lowercase dependency id, a lowercase done status, and a non-Done terminal column

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.01 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
