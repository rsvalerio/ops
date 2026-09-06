---
id: TASK-1685
title: code-review-plan-wave143
status: Done
assignee:
  - code-review-wave
created_date: '2026-08-26 21:17'
updated_date: '2026-08-26 22:02'
labels:
  - code-review-wave
dependencies:
  - TASK-1675
  - TASK-1682
modified_files:
  - Cargo.toml
  - crates/cli/src/args.rs
  - crates/cli/src/extension_cmd.rs
  - crates/cli/src/help.rs
  - crates/cli/src/import_makefile_cmd.rs
  - crates/cli/src/main.rs
  - crates/cli/src/test_utils.rs
  - crates/cli/src/theme_cmd.rs
  - crates/cli/tests/integration.rs
  - crates/core/src/config/commands.rs
  - crates/core/src/config/loader/global.rs
  - crates/core/src/config/theme_types.rs
  - crates/core/src/project_identity/card.rs
  - crates/core/src/stack/mod.rs
  - crates/core/src/subprocess/drain.rs
  - crates/extension/src/data.rs
  - crates/runner/src/command/mod.rs
  - crates/runner/src/command/parallel.rs
  - crates/runner/src/command/tests/build_cmd.rs
  - crates/runner/src/command/tests/events.rs
  - crates/runner/src/command/tests/exec.rs
  - crates/runner/src/command/tests/parallel.rs
  - crates/runner/src/command/tests/parallel_infra.rs
  - crates/runner/src/command/tests/sequential.rs
  - crates/runner/src/display.rs
  - crates/runner/src/display/finalize.rs
  - crates/runner/src/display/style.rs
  - crates/runner/src/display/tests.rs
  - crates/theme/src/tests/deserialize.rs
  - extensions-java/about/src/maven/pom.rs
  - extensions-node/about/src/lib.rs
  - extensions-node/about/src/units.rs
  - extensions-python/about/src/lib.rs
  - extensions-rust/about/src/query.rs
  - extensions-rust/cargo-toml/src/inheritance.rs
  - extensions-rust/cargo-toml/src/types.rs
  - extensions-rust/metadata/src/tests/edge_cases.rs
  - extensions/about/src/manifest_cache.rs
  - extensions/about/src/workspace.rs
  - extensions/duckdb/src/sql/ingest/orchestrator.rs
  - extensions/duckdb/src/sql/query/coverage.rs
  - extensions/git/src/config.rs
  - extensions/hook-common/src/git_state.rs
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave143
<!-- SECTION:DESCRIPTION:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
The residual small allows: `expect_used` plus the fourteen-lint tail (which carries the other explicit-panic lints, `unreachable` and `panic_in_result_fn`). One pass drops 15 lines from the temporary-allow block.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-1683 [wave141] 15 files (Cargo.toml, crates/cli/src/args.rs...); TASK-1684 [wave142] 18 files (Cargo.toml, crates/cli/src/help.rs...); TASK-1686 [wave144] 22 files (Cargo.toml, crates/cli/src/args.rs...); TASK-1687 [wave145] 10 files (Cargo.toml, crates/cli/src/args.rs...); TASK-1688 [wave146] 7 files (Cargo.toml, crates/core/src/config/loader/global.rs...); TASK-1689 [wave147] 4 files (Cargo.toml, extensions-rust/about/src/query.rs...)

Every wave in this batch edits the `# --- Temporary allows ---` block in the root `Cargo.toml`, so a one-line merge there is expected on each landing.

Branch: code-review/TASK-1685
<!-- SECTION:NOTES:END -->
