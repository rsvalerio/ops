---
id: TASK-2648
title: 'ops foundation fails on the ops repo itself: rustfmt.toml missing'
status: Done
assignee: []
created_date: '2026-11-01 00:09'
updated_date: '2026-11-01 13:55'
labels:
  - code-review-rust
  - config
dependencies: []
parent_task_id: 'TASK-2657'
modified_files:
  - rustfmt.toml
  - .ops.toml
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `rustfmt.toml`

**What**: `ops init --rust --check`, and so the new `ops foundation`, exits 1 at the repo root with `drift rustfmt.toml: missing`. The repo has no rustfmt.toml and no `[foundation.waivers]` entry for it. Adopting the template reformats the tree, so it needs a decision, not a drive-by.

**Why it matters**: `foundation` cannot join this repo's own `qa` while the template's source repo drifts from it.

**Origin**: discovered during TASK-2645 while fixing TASK-2640.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ops foundation exits 0 on this repo, by adopting rustfmt.toml or a recorded waiver

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decision (2026-11-01, user): adopt rustfmt.toml from the template rather than a waiver. Expect a reformat of the tree.
<!-- SECTION:NOTES:END -->
