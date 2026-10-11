---
id: TASK-2463
title: 'READ-13: Cargo.toml comment carries a task-provenance tag'
status: Done
assignee: []
created_date: '2026-10-10 15:27'
updated_date: '2026-10-10 22:10'
labels:
  - code-review
  - readability
dependencies: []
parent_task_id: 'TASK-2621'
modified_files:
  - extensions-node/about/Cargo.toml
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-node/about/Cargo.toml:ops-about-node'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/Cargo.toml:12`

**What**: The comment above the `linkme` dependency opens with a task-provenance tag - `ARCH-11 (TASK-2013):` - before its load-bearing explanation of why the dependency has no textual use in this crate.

**Why it matters**: READ-13 - task IDs and rule-provenance tags are process artifacts meaningless to a reader of the manifest; they also go stale when the backlog task is closed or renumbered. The same shape was already cleaned in ops core docs (commits 1d9bc644, e5b5443f) and filed as TASK-2442 for another crate. The explanation itself (impl_extension! expands to a distributed_slice attribute that resolves in the calling crate) must stay - only the tag goes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The TASK-2013 provenance tag is removed from the comment; the explanation of why linkme is a load-bearing dependency remains

<!-- AC:END -->
