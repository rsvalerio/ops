---
id: TASK-2537
title: 'READ-13: wave_create module docs narrate the N+1 workflow this command replaced'
status: To Do
assignee: []
created_date: '2026-10-10 15:36'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2613'
modified_files:
  - crates/backlog/src/cmd/wave_create.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:crates/backlog/src/cmd/wave_create.rs:wave_create'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/cmd/wave_create.rs:1-8` (module docs)

**What**: The module doc opens with journey narration: "Triage used to do this as one `task create` plus one `task edit` per member — N+1 separate writes, any of which an agent could skip or get wrong (a member missing its `parent_task_id`, or left in `Triage`). This command writes every link ... at once". That is the design history of the change, not a property of the API. What the reader needs is already in the second half: what one invocation writes (parent's marker label and `dependencies:`, each member's `parent_task_id` and status) and the all-checks-before-first-write guarantee from the run_wave_create docs.

**Why it matters**: Per READ-13, docs describe the end state, not the journey; "used to do X" paragraphs go stale on the next change while looking authoritative, and mean nothing to a reader who never saw the old workflow. The same commit-narration cleanup was already applied to other crates (e5b5443f, 1d9bc644).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Module docs state only what the command writes and its failure ordering; no pre-command workflow narration
- [ ] #2 The writes-every-link-at-once semantics remain documented on the command itself
<!-- AC:END -->
