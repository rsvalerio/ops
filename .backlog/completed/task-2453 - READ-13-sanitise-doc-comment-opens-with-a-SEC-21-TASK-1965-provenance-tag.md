---
id: TASK-2453
title: 'READ-13: sanitise doc comment opens with a SEC-21 / TASK-1965 provenance tag'
status: Done
assignee: []
created_date: '2026-10-10 15:24'
updated_date: '2026-10-10 21:25'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2609'
modified_files:
  - crates/theme/src/render.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:crates/theme/src/render.rs:sanitise'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/render.rs:8`

**What**: The `///` doc on `pub fn sanitise` opens with `SEC-21 / TASK-1965: neutralise a string that came from a child process...` — a guideline/task self-report prefix. The rest of the doc is durable end-state documentation (what is neutralised and why routing through `ops_core::ui::sanitise_line` matters); only the tag is journey narration.

**Why it matters**: READ-13 — self-report tags naming which guideline a change followed are process artifacts; a reader using the API gets nothing from the identifier and it goes stale on the next change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The sanitise doc summary states the behaviour without the SEC-21 / TASK-1965 prefix; the threat-model rationale is kept

<!-- AC:END -->
