---
id: TASK-1671
title: 'CLIPPY: drop the workspace allow for `arithmetic_side_effects`'
status: Triage
assignee: []
created_date: '2026-08-25 21:00'
labels:
  - code-review-rust
  - clippy
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Enabling `clippy::nursery` and the panic/arithmetic lints from the photo config surfaced 166 pre-existing sites for this lint family across 54 files. To keep `cargo clippy --workspace --all-features --all-targets -- -D warnings` green, the lint is currently allowed workspace-wide in the `# --- Temporary allows ---` block of the root `Cargo.toml`. That allow is the thing this task removes.

**Lint(s)**: `clippy::arithmetic_side_effects`
**Sites**: 166 across 54 files

Every `+ - * / %` and `+=` on integers that can wrap, overflow or divide by zero. Not mechanical: each site needs a decision — `checked_*` with a real error path, `saturating_*`, or an `#[allow]` at the statement with a comment proving the bound cannot be exceeded. Blanket `saturating_*` is the wrong answer where wrapping means a wrong number reaches the user (widths, counts, byte offsets).

## Scope

| File | Sites |
|---|---|
| `crates/core/src/output.rs` | 15 |
| `crates/theme/src/configurable.rs` | 13 |
| `crates/runner/src/command/secret_patterns.rs` | 8 |
| `extensions-rust/loc/src/counter.rs` | 8 |
| `extensions-java/about/src/maven/pom.rs` | 7 |
| `extensions/about/src/text_util.rs` | 7 |
| `extensions-rust/deps/src/parse/upgrade.rs` | 6 |
| `extensions/text-fixers/src/eof.rs` | 6 |
| `extensions/text-fixers/src/trailing.rs` | 6 |
| `crates/core/src/subprocess/drain.rs` | 4 |
| `crates/runner/src/command/exec.rs` | 4 |
| `extensions-go/about/src/go_syntax.rs` | 4 |
| `extensions-rust/about/src/query.rs` | 4 |
| `extensions/config-checkers/src/lib.rs` | 4 |
| `extensions/git/src/config.rs` | 4 |
| `crates/core/src/text.rs` | 3 |
| `crates/runner/src/command/resolve.rs` | 3 |
| `crates/runner/src/display.rs` | 3 |
| `extensions-java/about/src/gradle/lexer.rs` | 3 |
| `extensions-rust/cargo-update/src/lib.rs` | 3 |
| `extensions-rust/deps/src/format.rs` | 3 |
| `extensions-terraform/about/src/lib.rs` | 3 |
| `extensions/git/src/remote.rs` | 3 |
| `crates/cli/src/help.rs` | 2 |
| `crates/cli/src/import_makefile_cmd.rs` | 2 |
| `crates/core/src/config/commands.rs` | 2 |
| `crates/core/src/project_identity/card.rs` | 2 |
| `crates/runner/src/command/parallel.rs` | 2 |
| `crates/runner/src/display/finalize.rs` | 2 |
| `extensions-rust/test-coverage/src/parse.rs` | 2 |
| `extensions-terraform/plan/src/render.rs` | 2 |
| `extensions/about/src/workspace.rs` | 2 |
| `extensions/duckdb/src/sql/validation.rs` | 2 |
| `extensions/hook-common/src/git.rs` | 2 |
| `crates/cli/src/run_cmd/dry_run.rs` | 1 |
| `crates/cli/tests/integration.rs` | 1 |
| `crates/core/src/config/edit.rs` | 1 |
| `crates/core/src/config/loader/env.rs` | 1 |
| `crates/core/src/config/root.rs` | 1 |
| `crates/core/src/expand.rs` | 1 |
| `crates/core/src/project_identity/format.rs` | 1 |
| `crates/core/src/test_utils.rs` | 1 |
| `crates/core/src/ui.rs` | 1 |
| `crates/runner/src/command/tests/expand.rs` | 1 |
| `crates/runner/src/display/progress_state.rs` | 1 |
| `crates/theme/src/render.rs` | 1 |
| `extensions-go/about/src/go_mod.rs` | 1 |
| `extensions-go/about/src/lib.rs` | 1 |
| `extensions-node/about/src/package_json.rs` | 1 |
| `extensions-node/about/src/units.rs` | 1 |
| `extensions-rust/deps/src/parse/mod.rs` | 1 |
| `extensions/duckdb/src/sql/query/deps.rs` | 1 |
| `extensions/duckdb/src/sql/query/helpers.rs` | 1 |
| `extensions/text-fixers/src/lib.rs` | 1 |
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every site listed in the scope table is either fixed or carries an `#[allow]` at the narrowest scope that works, with a comment giving the reason (docs/clippy.md layer 2 or 3)
- [ ] #2 The line(s) for `arithmetic_side_effects` are deleted from the temporary-allow block in the root `Cargo.toml`, and the lint reaches the workspace at `deny`
- [ ] #3 `cargo clippy --workspace --all-features --all-targets -- -D warnings` passes
- [ ] #4 `cargo nextest run --workspace --all-features` and `cargo test --workspace --doc` pass
<!-- AC:END -->
