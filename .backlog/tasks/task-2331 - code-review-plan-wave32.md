---
id: TASK-2331
title: 'code-review-plan-wave32'
status: Done
assignee: []
created_date: '2026-09-28 15:07'
updated_date: '2026-09-28 15:29'
labels:
  - code-review-wave
dependencies:
  - TASK-2322
  - TASK-2323
  - TASK-2324
modified_files:
  - crates/core/src/.default.rust.ops.toml
  - extensions/text-fixers/src/options.rs
  - extensions/text-fixers/src/runner.rs
  - extensions-rust/deps/src/lib.rs
  - crates/cli/src/args.rs
  - crates/cli/src/subcommands.rs
  - .github/workflows/ci.yml
  - docs/commands.md
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave32
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Rationale: CI-safe Rust stack gates — non-mutating fmt/tw/eof check, --locked defaults, deps check mode. Scope is predicted (feature tasks carry no --modified-file); new source files may be added.
Overlaps: TASK-2333/wave34 (crates/cli/src/args.rs, extensions-rust/deps/src/lib.rs, docs/commands.md); TASK-2334/wave35 (crates/cli/src/args.rs, crates/cli/src/subcommands.rs, .github/workflows/ci.yml, docs/commands.md)

Branch: code-review/TASK-2331

<!-- SECTION:NOTES:END -->
