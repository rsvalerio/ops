---
id: TASK-2203
title: 'PATTERN-1: about-python hardcodes module_count = None, so the ''packages'' card row is blank for uv workspaces the units provider does resolve'
status: Done
assignee: []
created_date: '2026-09-08 07:19'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - pattern
dependencies: []
parent_task_id: 'TASK-2239'
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
- [x] #1 module_count for a uv workspace equals the number of ProjectUnits the units provider emits for the same root, or the module_label is dropped if no count is intended
- [x] #2 A single-package (non-workspace) pyproject still yields module_count = None
- [x] #3 A test pins the count against the units list for the same fixture so the two cannot drift
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed: module_count now derives from the same resolved member set the units provider lists. units::read_workspace_members is exposed (pub within the private module) and the identity provider sets module_count = Some(len) for a uv workspace that resolves members, None for single-package or zero-resolution projects (label kept — the count fills it). Joint test uv_workspace_module_count_equals_the_units_provider_length pins identity module_count == PythonUnitsProvider units len on one fixture including a non-resolving member dir; parse_minimal_pyproject now also pins the single-package None. cargo test -p ops-about-python: 47 passed; clippy pedantic clean.
<!-- SECTION:NOTES:END -->
