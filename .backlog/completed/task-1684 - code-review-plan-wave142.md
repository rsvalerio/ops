---
id: TASK-1684
title: code-review-plan-wave142
status: Done
assignee:
  - code-review-wave
created_date: '2026-08-26 21:17'
updated_date: '2026-08-26 22:57'
labels:
  - code-review-wave
dependencies:
  - TASK-1671
  - TASK-1674
modified_files:
  - Cargo.toml
  - crates/cli/src/help.rs
  - crates/cli/src/import_makefile_cmd.rs
  - crates/cli/src/run_cmd/dry_run.rs
  - crates/cli/tests/integration.rs
  - crates/core/src/config/commands.rs
  - crates/core/src/config/edit.rs
  - crates/core/src/config/loader/env.rs
  - crates/core/src/config/loader/mod.rs
  - crates/core/src/config/root.rs
  - crates/core/src/expand.rs
  - crates/core/src/output.rs
  - crates/core/src/project_identity/card.rs
  - crates/core/src/project_identity/format.rs
  - crates/core/src/stack/metadata.rs
  - crates/core/src/subprocess/cap.rs
  - crates/core/src/subprocess/drain.rs
  - crates/core/src/test_utils.rs
  - crates/core/src/text.rs
  - crates/core/src/ui.rs
  - crates/runner/src/command/exec.rs
  - crates/runner/src/command/parallel.rs
  - crates/runner/src/command/resolve.rs
  - crates/runner/src/command/results.rs
  - crates/runner/src/command/secret_patterns.rs
  - crates/runner/src/command/tests/exec.rs
  - crates/runner/src/command/tests/expand.rs
  - crates/runner/src/command/tests/parallel.rs
  - crates/runner/src/command/tests/sequential.rs
  - crates/runner/src/display.rs
  - crates/runner/src/display/finalize.rs
  - crates/runner/src/display/progress_state.rs
  - crates/theme/src/configurable.rs
  - crates/theme/src/render.rs
  - crates/theme/src/step_line_theme.rs
  - crates/theme/src/tests/boxed_layout.rs
  - crates/theme/src/tests/format_duration.rs
  - crates/theme/src/tests/render_report.rs
  - extensions-go/about/src/go_mod.rs
  - extensions-go/about/src/go_syntax.rs
  - extensions-go/about/src/lib.rs
  - extensions-java/about/src/gradle/lexer.rs
  - extensions-java/about/src/maven/pom.rs
  - extensions-node/about/src/package_json.rs
  - extensions-node/about/src/units.rs
  - extensions-rust/about/src/query.rs
  - extensions-rust/cargo-update/src/lib.rs
  - extensions-rust/deps/src/format.rs
  - extensions-rust/deps/src/parse/mod.rs
  - extensions-rust/deps/src/parse/upgrade.rs
  - extensions-rust/loc/src/counter.rs
  - extensions-rust/test-coverage/src/parse.rs
  - extensions-terraform/about/src/lib.rs
  - extensions-terraform/plan/src/lib.rs
  - extensions-terraform/plan/src/render.rs
  - extensions/about/src/loc.rs
  - extensions/about/src/manifest_io.rs
  - extensions/about/src/text_util.rs
  - extensions/about/src/workspace.rs
  - extensions/config-checkers/src/lib.rs
  - extensions/duckdb/src/schema.rs
  - extensions/duckdb/src/sql/ingest/dir.rs
  - extensions/duckdb/src/sql/ingest/sidecar.rs
  - extensions/duckdb/src/sql/query/deps.rs
  - extensions/duckdb/src/sql/query/helpers.rs
  - extensions/duckdb/src/sql/validation.rs
  - extensions/git/src/config.rs
  - extensions/git/src/remote.rs
  - extensions/hook-common/src/git.rs
  - extensions/text-fixers/src/eof.rs
  - extensions/text-fixers/src/lib.rs
  - extensions/text-fixers/src/trailing.rs
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave142
<!-- SECTION:DESCRIPTION:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Silent integer corruption: `arithmetic_side_effects` + `as_conversions`. Both are per-site decisions about numbers going wrong quietly, fixed with the same `checked_*`/`try_into` toolkit in the same widths/counts/offsets code.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-1683 [wave141] 35 files (Cargo.toml, crates/cli/src/help.rs...); TASK-1685 [wave143] 18 files (Cargo.toml, crates/cli/src/help.rs...); TASK-1686 [wave144] 36 files (Cargo.toml, crates/cli/src/help.rs...); TASK-1687 [wave145] 19 files (Cargo.toml, crates/cli/src/help.rs...); TASK-1688 [wave146] 17 files (Cargo.toml, crates/core/src/config/edit.rs...); TASK-1689 [wave147] 5 files (Cargo.toml, crates/core/src/expand.rs...)

Every wave in this batch edits the `# --- Temporary allows ---` block in the root `Cargo.toml`, so a one-line merge there is expected on each landing.

Branch: code-review/TASK-1684
<!-- SECTION:NOTES:END -->
