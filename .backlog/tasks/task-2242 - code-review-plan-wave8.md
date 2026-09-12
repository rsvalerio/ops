---
id: TASK-2242
title: 'code-review-plan-wave8'
status: Done
assignee: []
created_date: '2026-09-08 10:51'
updated_date: '2026-09-10 18:38'
labels:
  - code-review-wave
dependencies:
  - TASK-2148
  - TASK-2150
  - TASK-2106
  - TASK-2111
  - TASK-2077
  - TASK-2183
  - TASK-2193
  - TASK-2196
  - TASK-2211
  - TASK-2220
modified_files:
  - crates/backlog/src/cmd/create.rs
  - crates/backlog/src/cmd/edit.rs
  - crates/theme/src/style/strip.rs
  - extensions-go/about/src/go_mod.rs
  - extensions-python/about/src/units.rs
  - extensions-rust/about/src/coverage_provider.rs
  - extensions-rust/about/src/manifest_cache.rs
  - extensions-rust/about/src/workspace_root_cache.rs
  - extensions-rust/cargo-update/src/lib.rs
  - extensions-rust/deps/src/format.rs
  - extensions-rust/loc/src/lib.rs
  - extensions-terraform/about/src/lib.rs
  - extensions/create-review-tasks/src/backlog.rs
  - extensions/tokei/src/lib.rs
ordinal: 148000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave8: Copy-pasted logic that has already diverged between its copies
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-2248 wave14 (6 files: extensions-go/about/src/go_mod.rs ...); TASK-2247 wave13 (5 files: extensions-go/about/src/go_mod.rs ...); TASK-2246 wave12 (3 files: crates/backlog/src/cmd/edit.rs ...); TASK-2235 wave1 (2 files: crates/backlog/src/cmd/edit.rs ...); TASK-2238 wave4 (2 files: extensions-go/about/src/go_mod.rs ...); TASK-2239 wave5 (2 files: extensions-python/about/src/units.rs ...); TASK-2240 wave6 (2 files: extensions-python/about/src/units.rs ...); TASK-2243 wave9 (2 files: extensions-rust/cargo-update/src/lib.rs ...); TASK-2244 wave10 (2 files: extensions/create-review-tasks/src/backlog.rs ...); TASK-2236 wave2 (1 file: extensions-terraform/about/src/lib.rs); TASK-2241 wave7 (1 file: extensions-python/about/src/units.rs)

Branch: code-review/TASK-2242

Rebase onto code-review/run-20260908 (77f107b8) hit one conflict in extensions-python/about/src/units.rs: the landing branch had expanded invalid_root_pyproject_yields_no_units with capture_tracing warn assertions while this wave renamed the local write helper to the shared write_file. Resolved by keeping the landed assertions in full and applying only the write -> write_file rename. Integration gates on the merged result: ops verify 8/8, ops qa 4/4, cargo nextest --run-ignored all 3266/3266, doctests 1/1.

<!-- SECTION:NOTES:END -->
