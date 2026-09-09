---
id: TASK-2241
title: 'code-review-plan-wave7'
status: To Do
assignee: []
created_date: '2026-09-08 10:51'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-wave
dependencies:
  - TASK-2076
  - TASK-2213
  - TASK-2118
  - TASK-2200
  - TASK-2205
  - TASK-2096
  - TASK-2091
  - TASK-2119
  - TASK-2144
  - TASK-2113
  - TASK-2142
  - TASK-2133
modified_files:
  - crates/extension/src/lib.rs
  - crates/extension/src/tests.rs
  - crates/runner/src/command/results.rs
  - crates/theme/src/configurable.rs
  - extensions-python/about/src/units.rs
  - extensions-rust/test-coverage/src/tests/views.rs
  - extensions-terraform/plan/src/lib.rs
  - extensions/git/src/provider.rs
  - extensions/run-before-commit/src/lib.rs
  - extensions/run-before-push/src/lib.rs
ordinal: 147000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave7: Env/cwd races, vacuous preconditions, and suite placement
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-2248 wave14 (6 files: crates/extension/src/lib.rs ...); TASK-2247 wave13 (4 files: extensions-terraform/plan/src/lib.rs ...); TASK-2234 wave0 (2 files: extensions/run-before-commit/src/lib.rs ...); TASK-2246 wave12 (2 files: crates/extension/src/lib.rs ...); TASK-2235 wave1 (1 file: extensions-terraform/plan/src/lib.rs); TASK-2239 wave5 (1 file: extensions-python/about/src/units.rs); TASK-2240 wave6 (1 file: extensions-python/about/src/units.rs); TASK-2242 wave8 (1 file: extensions-python/about/src/units.rs); TASK-2244 wave10 (1 file: crates/theme/src/configurable.rs); TASK-2245 wave11 (1 file: crates/extension/src/lib.rs)
<!-- SECTION:NOTES:END -->
