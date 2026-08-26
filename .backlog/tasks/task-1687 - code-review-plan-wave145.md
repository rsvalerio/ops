---
id: TASK-1687
title: code-review-plan-wave145
status: To Do
assignee:
  - code-review-wave
created_date: '2026-08-26 21:17'
updated_date: '2026-08-26 21:18'
labels:
  - code-review-wave
dependencies:
  - TASK-1680
modified_files:
  - Cargo.toml
  - crates/cli/src/about_cmd.rs
  - crates/cli/src/args.rs
  - crates/cli/src/help.rs
  - crates/cli/src/registry/discovery.rs
  - crates/cli/src/registry/registration.rs
  - crates/cli/src/subcommands.rs
  - crates/core/src/expand.rs
  - crates/core/src/project_identity/card.rs
  - crates/core/src/stack/mod.rs
  - crates/core/src/subprocess/cap.rs
  - crates/core/src/text.rs
  - crates/runner/src/command/exec.rs
  - crates/runner/src/command/mod.rs
  - crates/runner/src/command/results.rs
  - crates/runner/src/display.rs
  - crates/runner/src/display/finalize.rs
  - crates/theme/src/style/sgr.rs
  - extensions-go/about/src/modules.rs
  - extensions-java/about/src/gradle/lexer.rs
  - extensions-rust/about/src/units.rs
  - extensions-rust/cargo-toml/src/lib.rs
  - extensions-rust/cargo-update/src/lib.rs
  - extensions-rust/metadata/src/types.rs
  - extensions-rust/test-coverage/src/parse.rs
  - extensions-rust/test-coverage/src/subprocess.rs
  - extensions-terraform/plan/src/lib.rs
  - extensions/about/src/cards.rs
  - extensions/about/src/workspace.rs
  - extensions/duckdb/src/connection.rs
  - extensions/duckdb/src/sql/ingest/sql.rs
  - extensions/duckdb/src/sql/query/helpers.rs
  - extensions/git/src/config.rs
  - extensions/git/src/remote.rs
  - extensions/hook-common/src/paths.rs
  - extensions/text-fixers/src/trailing.rs
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave145
<!-- SECTION:DESCRIPTION:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
`option_if_let_else` alone: per-site readability judgement in expression bodies, unrelated to the declaration-shape and panic-safety work.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-1683 [wave141] 18 files (Cargo.toml, crates/cli/src/args.rs...); TASK-1684 [wave142] 19 files (Cargo.toml, crates/cli/src/help.rs...); TASK-1685 [wave143] 10 files (Cargo.toml, crates/cli/src/args.rs...); TASK-1686 [wave144] 20 files (Cargo.toml, crates/cli/src/args.rs...); TASK-1688 [wave146] 9 files (Cargo.toml, crates/core/src/stack/mod.rs...); TASK-1689 [wave147] 5 files (Cargo.toml, crates/core/src/expand.rs...)

Every wave in this batch edits the `# --- Temporary allows ---` block in the root `Cargo.toml`, so a one-line merge there is expected on each landing.
<!-- SECTION:NOTES:END -->
