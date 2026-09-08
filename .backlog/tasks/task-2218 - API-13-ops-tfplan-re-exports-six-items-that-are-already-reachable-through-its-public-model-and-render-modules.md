---
id: TASK-2218
title: >-
  API-13: ops-tfplan re-exports six items that are already reachable through its
  public model and render modules
status: To Do
assignee:
  - TASK-2247
created_date: '2026-09-08 07:21'
updated_date: '2026-09-08 11:00'
labels:
  - code-review-rust
  - api
dependencies: []
modified_files:
  - extensions-terraform/plan/src/lib.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:11-15`

**What**:

    pub mod model;
    pub mod render;

    pub use model::{Action, ClassifiedChange, Plan};
    pub use render::{render_outputs_table, render_resource_table, render_summary_table};

Both modules stay `pub`, so every one of those six items has two public paths: `ops_tfplan::Action` and `ops_tfplan::model::Action`, `ops_tfplan::render_summary_table` and `ops_tfplan::render::render_summary_table`. Rustdoc lists both, downstream code picks arbitrarily, and the two paths cannot be evolved independently — moving `Action` out of `model` is a breaking change on a path that was supposed to be an implementation detail.

Note the export is also incomplete in a way that makes the boundary look accidental rather than curated: `ResourceChange` and `Change` are public in `model` but not re-exported, even though `Plan::resource_changes` returns them, so a caller who takes the flat path still has to reach into `model` for the types it hands back.

**Why it matters**: The crate has no stated policy on which path is the supported one. Pick one — either the flat facade (make `model` and `render` `pub(crate)` and re-export the complete set, including `ResourceChange` and `Change`) or the module paths (drop the `pub use` lines). Either is defensible; having both is what costs, because every future move of an item is a breaking change on whichever path the crate did not intend to support.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each public item of ops-tfplan is reachable by exactly one public path
- [ ] #2 If the flat facade is chosen, the re-export set is complete — types reachable from a re-exported type's public fields (ResourceChange, Change) are re-exported too
- [ ] #3 In-tree callers are updated to the chosen path
<!-- AC:END -->
