---
id: TASK-2245
title: code-review-plan-wave11
status: Done
assignee:
  - code-review-wave
created_date: '2026-09-08 10:52'
updated_date: '2026-09-08 16:57'
labels:
  - code-review-wave
dependencies:
  - TASK-2085
  - TASK-2102
  - TASK-2104
  - TASK-2087
  - TASK-2094
modified_files:
  - .github/workflows/ci.yml
  - crates/backlog/Cargo.toml
  - crates/core/src/config/edit.rs
  - crates/core/src/text.rs
  - crates/extension/src/lib.rs
  - crates/runner/src/command/process_group.rs
  - crates/runner/src/terminal.rs
  - crates/theme/Cargo.toml
ordinal: 151000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave11: Mechanical unsafe forbiddance and Miri coverage for the FFI that exists
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-2241 wave7 (1 file: crates/extension/src/lib.rs); TASK-2246 wave12 (1 file: crates/extension/src/lib.rs); TASK-2248 wave14 (1 file: crates/extension/src/lib.rs)

Branch: code-review/TASK-2245
<!-- SECTION:NOTES:END -->
