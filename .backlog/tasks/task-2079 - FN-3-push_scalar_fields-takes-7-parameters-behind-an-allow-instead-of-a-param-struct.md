---
id: TASK-2079
title: 'FN-3: push_scalar_fields takes 7 parameters behind an allow instead of a param struct'
status: Done
assignee: []
created_date: '2026-09-07 22:57'
updated_date: '2026-09-09 18:25'
labels:
  - code-review-rust
  - structure
dependencies: []
parent_task_id: 'TASK-2246'
modified_files:
  - crates/backlog/src/render.rs
priority: medium
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/render.rs:300`

**What**: `push_scalar_fields(s, entry, root, readiness, task_type, parent, ordinal)` takes 7 parameters and carries `#[allow(clippy::too_many_arguments)]` to silence the mechanical check. `task_type: Option<&str>` and `parent: Option<&str>` are adjacent same-typed parameters — the classic FN-3 mix-up hazard (both are extras-derived scalars computed one screen earlier in `view_json`).

**Why it matters**: The allow converts a compiler-checked limit into a comment; a swapped `task_type`/`parent` pair still compiles and silently emits the wrong `type`/`parentTaskId` JSON fields. The three trailing params are exactly the trio `view_json` already extracts from `frontmatter.extras` — grouping them (with `ordinal`) into a small struct names the dependency and makes the swap unrepresentable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 push_scalar_fields takes at most 5 parameters, or the extras-derived values travel in one named struct
- [x] #2 The #[allow(clippy::too_many_arguments)] is gone

<!-- AC:END -->
