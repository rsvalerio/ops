---
id: TASK-2203
title: >-
  PATTERN-1: about-python hardcodes module_count = None, so the 'packages' card
  row is blank for uv workspaces the units provider does resolve
status: To Do
assignee:
  - TASK-2239
created_date: '2026-09-08 07:19'
updated_date: '2026-09-08 10:55'
labels:
  - code-review-rust
  - pattern
dependencies: []
modified_files:
  - extensions-python/about/src/lib.rs
  - extensions-python/about/src/units.rs
priority: medium
ordinal: 116000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:107`, `extensions-python/about/src/units.rs:124`

**What**: The identity provider sets `m.module_label = "packages"` and `m.module_count = None` unconditionally. `crates/core/src/project_identity/card.rs:80` renders the modules row as `label = module_label`, `value = module_count.map(...)` — so the row carries a label and no value, for every Python project. Meanwhile `units::collect_units` resolves `[tool.uv.workspace].members` globs and emits N `ProjectUnit`s, which the workspace card lists. The count and the table are read off different data and disagree by construction.

`extensions-rust/about/src/identity/mod.rs:75` populates `module_count` from the same resolved workspace member list its units provider uses, which is the shape this crate should match.

**Why it matters**: `ops about` on a uv workspace with 8 packages shows a units table of 8 rows and a `packages` field with nothing in it. Declaring a `module_label` the provider can never fill is a stale invariant rather than a deliberate "not applicable" — if the count is genuinely not wanted, the label should go too.

**Twin**: TASK-2178 files the mirror image against `extensions-go/about` (count computed but diverging from the units list). `extensions-node/about/src/lib.rs:99` has the same `module_count = None` hardcode and should be resolved the same way.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 module_count for a uv workspace equals the number of ProjectUnits the units provider emits for the same root, or the module_label is dropped if no count is intended
- [ ] #2 A single-package (non-workspace) pyproject still yields module_count = None
- [ ] #3 A test pins the count against the units list for the same fixture so the two cannot drift
<!-- AC:END -->
