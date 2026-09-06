---
id: TASK-1686
title: code-review-plan-wave144
status: Done
assignee:
  - code-review-wave
created_date: '2026-08-26 21:17'
updated_date: '2026-08-26 22:28'
labels:
  - code-review-wave
dependencies:
  - TASK-1676
  - TASK-1677
  - TASK-1678
modified_files:
  - Cargo.toml
  - crates/cli/src/args.rs
  - crates/cli/src/extension_cmd.rs
  - crates/cli/src/help.rs
  - crates/cli/src/import_makefile_cmd.rs
  - crates/cli/src/init_cmd.rs
  - crates/cli/src/new_command_cmd.rs
  - crates/cli/src/row.rs
  - crates/cli/src/run_cmd.rs
  - crates/cli/src/run_cmd/dry_run.rs
  - crates/cli/src/run_cmd/plan.rs
  - crates/cli/src/sec_cmd.rs
  - crates/cli/src/subcommands.rs
  - crates/cli/src/test_utils.rs
  - crates/cli/src/theme_cmd.rs
  - crates/core/src/config/commands.rs
  - crates/core/src/config/edit.rs
  - crates/core/src/config/init.rs
  - crates/core/src/config/loader/global.rs
  - crates/core/src/config/loader/mod.rs
  - crates/core/src/config/sections.rs
  - crates/core/src/config/theme_types.rs
  - crates/core/src/output.rs
  - crates/core/src/project_identity.rs
  - crates/core/src/report.rs
  - crates/core/src/stack/detect.rs
  - crates/core/src/stack/metadata.rs
  - crates/core/src/stack/mod.rs
  - crates/core/src/subprocess/mod.rs
  - crates/core/src/sync.rs
  - crates/core/src/table.rs
  - crates/core/src/test_utils.rs
  - crates/extension/src/data.rs
  - crates/extension/src/extension.rs
  - crates/extension/src/tests.rs
  - crates/runner/src/command/abort.rs
  - crates/runner/src/command/build.rs
  - crates/runner/src/command/events.rs
  - crates/runner/src/command/exec.rs
  - crates/runner/src/command/mod.rs
  - crates/runner/src/command/parallel.rs
  - crates/runner/src/command/results.rs
  - crates/runner/src/command/secret_patterns.rs
  - crates/runner/src/command/tests/expand.rs
  - crates/runner/src/command/tests/mod.rs
  - crates/runner/src/command/tests/parallel.rs
  - crates/runner/src/display.rs
  - crates/runner/src/display/error_detail.rs
  - crates/runner/src/display/progress_state.rs
  - crates/runner/src/display/render_config.rs
  - crates/runner/src/display/tap.rs
  - crates/runner/src/terminal.rs
  - crates/theme/src/configurable.rs
  - extensions-go/about/src/go_mod.rs
  - extensions-go/about/src/go_syntax.rs
  - extensions-go/about/src/go_work.rs
  - extensions-go/about/src/modules.rs
  - extensions-java/about/src/gradle/mod.rs
  - extensions-java/about/src/maven/mod.rs
  - extensions-node/about/src/package_json.rs
  - extensions-node/about/src/package_manager.rs
  - extensions-node/about/src/repo_url.rs
  - extensions-node/about/src/units.rs
  - extensions-python/about/src/units.rs
  - extensions-rust/about/src/coverage_provider.rs
  - extensions-rust/about/src/deps_provider.rs
  - extensions-rust/about/src/identity/mod.rs
  - extensions-rust/about/src/query.rs
  - extensions-rust/about/src/units.rs
  - extensions-rust/cargo-toml/src/inheritance.rs
  - extensions-rust/cargo-toml/src/lib.rs
  - extensions-rust/cargo-toml/src/types.rs
  - extensions-rust/cargo-toml/src/workspace_root.rs
  - extensions-rust/cargo-update/src/tests.rs
  - extensions-rust/deps/src/format.rs
  - extensions-rust/deps/src/parse/deny.rs
  - extensions-rust/deps/src/parse/mod.rs
  - extensions-rust/deps/src/test_support.rs
  - extensions-rust/loc/src/counter.rs
  - extensions-rust/metadata/src/test_support.rs
  - extensions-rust/metadata/src/types.rs
  - extensions-rust/metadata/src/views.rs
  - extensions-rust/test-coverage/src/parse.rs
  - extensions-rust/test-coverage/src/provider.rs
  - extensions-rust/test-coverage/src/subprocess.rs
  - extensions-rust/test-coverage/src/tests.rs
  - extensions-rust/test-coverage/src/views.rs
  - extensions-terraform/about/src/lib.rs
  - extensions-terraform/plan/src/model.rs
  - extensions/about/src/lib.rs
  - extensions/about/src/lru.rs
  - extensions/about/src/test_support.rs
  - extensions/config-checkers/src/lib.rs
  - extensions/duckdb/src/connection.rs
  - extensions/duckdb/src/error.rs
  - extensions/duckdb/src/ingestor.rs
  - extensions/duckdb/src/lib.rs
  - extensions/duckdb/src/schema.rs
  - extensions/duckdb/src/sql/ingest/dir.rs
  - extensions/duckdb/src/sql/ingest/orchestrator.rs
  - extensions/duckdb/src/sql/ingest/sql.rs
  - extensions/duckdb/src/sql/query/helpers.rs
  - extensions/git/src/config.rs
  - extensions/hook-common/src/fixtures.rs
  - extensions/hook-common/src/git.rs
  - extensions/hook-common/src/paths.rs
  - extensions/run-before-commit/src/lib.rs
  - extensions/text-fixers/src/lib.rs
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave144
<!-- SECTION:DESCRIPTION:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Item declaration shape: `redundant_pub_crate` + `missing_const_for_fn` + `use_self`. All three rewrite declarations and impl blocks and interact directly - visibility decides whether adding `const` is a semver-visible promise.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-1683 [wave141] 24 files (Cargo.toml, crates/cli/src/args.rs...); TASK-1684 [wave142] 36 files (Cargo.toml, crates/cli/src/help.rs...); TASK-1685 [wave143] 22 files (Cargo.toml, crates/cli/src/args.rs...); TASK-1687 [wave145] 20 files (Cargo.toml, crates/cli/src/args.rs...); TASK-1688 [wave146] 14 files (Cargo.toml, crates/core/src/config/edit.rs...); TASK-1689 [wave147] 10 files (Cargo.toml, crates/runner/src/command/build.rs...)

Every wave in this batch edits the `# --- Temporary allows ---` block in the root `Cargo.toml`, so a one-line merge there is expected on each landing.

Branch: code-review/TASK-1686
<!-- SECTION:NOTES:END -->
