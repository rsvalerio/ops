---
id: TASK-1677
title: 'CLIPPY: drop the workspace allow for `missing_const_for_fn` (nursery)'
status: To Do
assignee:
  - TASK-1686
created_date: '2026-08-25 21:00'
updated_date: '2026-08-26 21:18'
labels:
  - code-review-rust
  - clippy
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Enabling `clippy::nursery` and the panic/arithmetic lints from the photo config surfaced 92 pre-existing sites for this lint family across 44 files. To keep `cargo clippy --workspace --all-features --all-targets -- -D warnings` green, the lint is currently allowed workspace-wide in the `# --- Temporary allows ---` block of the root `Cargo.toml`. That allow is the thing this task removes.

**Lint(s)**: `clippy::missing_const_for_fn`
**Sites**: 92 across 44 files

Functions that could be `const fn`. Mechanical, but adding `const` is a semver-visible promise on public API — apply freely to private items and deliberately on the public surface.

## Scope

| File | Sites |
|---|---|
| `crates/core/src/config/sections.rs` | 6 |
| `extensions-rust/cargo-toml/src/types.rs` | 5 |
| `crates/cli/src/sec_cmd.rs` | 4 |
| `crates/core/src/test_utils.rs` | 4 |
| `crates/theme/src/configurable.rs` | 4 |
| `extensions-rust/cargo-toml/src/lib.rs` | 4 |
| `extensions-rust/deps/src/format.rs` | 4 |
| `extensions-terraform/plan/src/model.rs` | 4 |
| `extensions/config-checkers/src/lib.rs` | 4 |
| `crates/core/src/output.rs` | 3 |
| `crates/core/src/project_identity.rs` | 3 |
| `crates/extension/src/extension.rs` | 3 |
| `crates/runner/src/display/render_config.rs` | 3 |
| `extensions-rust/loc/src/counter.rs` | 3 |
| `extensions/duckdb/src/sql/query/helpers.rs` | 3 |
| `crates/core/src/config/init.rs` | 2 |
| `crates/extension/src/data.rs` | 2 |
| `crates/runner/src/command/mod.rs` | 2 |
| `crates/runner/src/display.rs` | 2 |
| `extensions/git/src/config.rs` | 2 |
| `extensions/text-fixers/src/lib.rs` | 2 |
| `crates/cli/src/args.rs` | 1 |
| `crates/cli/src/theme_cmd.rs` | 1 |
| `crates/core/src/config/loader/global.rs` | 1 |
| `crates/core/src/config/theme_types.rs` | 1 |
| `crates/core/src/report.rs` | 1 |
| `crates/core/src/stack/detect.rs` | 1 |
| `crates/core/src/stack/metadata.rs` | 1 |
| `crates/core/src/table.rs` | 1 |
| `crates/runner/src/command/events.rs` | 1 |
| `crates/runner/src/command/exec.rs` | 1 |
| `crates/runner/src/display/error_detail.rs` | 1 |
| `crates/runner/src/display/tap.rs` | 1 |
| `crates/runner/src/terminal.rs` | 1 |
| `extensions-go/about/src/go_mod.rs` | 1 |
| `extensions-rust/cargo-toml/src/workspace_root.rs` | 1 |
| `extensions-rust/test-coverage/src/parse.rs` | 1 |
| `extensions/about/src/lib.rs` | 1 |
| `extensions/about/src/lru.rs` | 1 |
| `extensions/duckdb/src/connection.rs` | 1 |
| `extensions/duckdb/src/ingestor.rs` | 1 |
| `extensions/duckdb/src/lib.rs` | 1 |
| `extensions/duckdb/src/schema.rs` | 1 |
| `extensions/duckdb/src/sql/ingest/dir.rs` | 1 |
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every site listed in the scope table is either fixed or carries an `#[allow]` at the narrowest scope that works, with a comment giving the reason (docs/clippy.md layer 2 or 3)
- [ ] #2 The line(s) for `missing_const_for_fn` are deleted from the temporary-allow block in the root `Cargo.toml`, and the lint reaches the workspace at `deny`
- [ ] #3 `cargo clippy --workspace --all-features --all-targets -- -D warnings` passes
- [ ] #4 `cargo nextest run --workspace --all-features` and `cargo test --workspace --doc` pass
<!-- AC:END -->
