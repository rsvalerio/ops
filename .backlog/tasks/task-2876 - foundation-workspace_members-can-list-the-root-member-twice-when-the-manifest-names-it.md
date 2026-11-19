---
id: TASK-2876
title: 'foundation workspace_members can list the root member twice when the manifest names it'
status: To Do
assignee: []
created_date: '2026-11-18 22:30'
updated_date: '2026-11-19 07:57'
labels:
  - code-review-rust
  - correctness
dependencies: []
parent_task_id: 'TASK-2883'
modified_files:
  - extensions-rust/foundation/src/lib.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/foundation/src/lib.rs:156` (`workspace_members`)

**What**: When the root manifest is also a package, `workspace_members` unconditionally inserts `"."` at the front of the resolved member list. `resolved_workspace_members` now emits every spelling of a root member (`.`, `./`, `.//`) as `ops_about_rust::ROOT_MEMBER` (`"."`), so a hybrid manifest that also lists the root yields `"."` twice.

**Why it matters**: The rust-foundation baseline then processes the root crate twice (duplicate opt-in work or output for one crate). Same root-dedup gap TASK-2746 fixed in create-review-tasks-rust.

**Origin**: discovered during TASK-2849 while fixing TASK-2746.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 workspace_members inserts the root only when the resolved list does not already contain ops_about_rust::ROOT_MEMBER, covered by a test listing the root as "./"
<!-- AC:END -->
