---
id: TASK-1676
title: 'CLIPPY: drop the workspace allow for `redundant_pub_crate` (nursery)'
status: Triage
assignee: []
created_date: '2026-08-25 21:00'
labels:
  - code-review-rust
  - clippy
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Enabling `clippy::nursery` and the panic/arithmetic lints from the photo config surfaced 195 pre-existing sites for this lint family across 60 files. To keep `cargo clippy --workspace --all-features --all-targets -- -D warnings` green, the lint is currently allowed workspace-wide in the `# --- Temporary allows ---` block of the root `Cargo.toml`. That allow is the thing this task removes.

**Lint(s)**: `clippy::redundant_pub_crate`
**Sites**: 195 across 60 files

Items marked `pub(crate)` inside a private module, where the visibility is a no-op. Mechanical: drop the `pub(crate)`. Largest single count in the sweep but the lowest risk — verify nothing was actually re-exported through a `pub use` before deleting.

## Scope

| File | Sites |
|---|---|
| `crates/runner/src/command/tests/mod.rs` | 28 |
| `extensions-rust/metadata/src/test_support.rs` | 12 |
| `crates/cli/src/subcommands.rs` | 11 |
| `crates/cli/src/help.rs` | 9 |
| `crates/runner/src/command/build.rs` | 6 |
| `extensions-rust/test-coverage/src/subprocess.rs` | 6 |
| `crates/cli/src/import_makefile_cmd.rs` | 5 |
| `extensions-rust/about/src/query.rs` | 5 |
| `extensions-rust/about/src/units.rs` | 5 |
| `extensions-rust/cargo-toml/src/inheritance.rs` | 5 |
| `extensions-rust/test-coverage/src/parse.rs` | 5 |
| `crates/cli/src/run_cmd.rs` | 4 |
| `crates/cli/src/run_cmd/plan.rs` | 4 |
| `crates/cli/src/sec_cmd.rs` | 4 |
| `crates/runner/src/command/parallel.rs` | 4 |
| `crates/runner/src/command/secret_patterns.rs` | 4 |
| `extensions-node/about/src/repo_url.rs` | 4 |
| `crates/cli/src/run_cmd/dry_run.rs` | 3 |
| `crates/core/src/config/loader/mod.rs` | 3 |
| `extensions-go/about/src/modules.rs` | 3 |
| `extensions-node/about/src/package_json.rs` | 3 |
| `extensions-rust/about/src/coverage_provider.rs` | 3 |
| `extensions-rust/deps/src/parse/mod.rs` | 3 |
| `extensions-rust/metadata/src/types.rs` | 3 |
| `extensions-rust/test-coverage/src/provider.rs` | 3 |
| `crates/cli/src/args.rs` | 2 |
| `crates/cli/src/row.rs` | 2 |
| `crates/cli/src/test_utils.rs` | 2 |
| `crates/core/src/config/edit.rs` | 2 |
| `crates/core/src/config/sections.rs` | 2 |
| `crates/core/src/sync.rs` | 2 |
| `extensions-go/about/src/go_mod.rs` | 2 |
| `extensions-go/about/src/go_syntax.rs` | 2 |
| `extensions-node/about/src/units.rs` | 2 |
| `extensions-python/about/src/units.rs` | 2 |
| `extensions-rust/about/src/deps_provider.rs` | 2 |
| `extensions-rust/about/src/identity/mod.rs` | 2 |
| `extensions-rust/test-coverage/src/views.rs` | 2 |
| `extensions/hook-common/src/fixtures.rs` | 2 |
| `extensions/hook-common/src/paths.rs` | 2 |
| `crates/cli/src/extension_cmd.rs` | 1 |
| `crates/cli/src/init_cmd.rs` | 1 |
| `crates/cli/src/new_command_cmd.rs` | 1 |
| `crates/core/src/config/commands.rs` | 1 |
| `crates/core/src/config/loader/global.rs` | 1 |
| `crates/runner/src/command/abort.rs` | 1 |
| `crates/runner/src/command/events.rs` | 1 |
| `crates/runner/src/command/results.rs` | 1 |
| `crates/runner/src/display/progress_state.rs` | 1 |
| `crates/runner/src/display/tap.rs` | 1 |
| `extensions-go/about/src/go_work.rs` | 1 |
| `extensions-java/about/src/gradle/mod.rs` | 1 |
| `extensions-java/about/src/maven/mod.rs` | 1 |
| `extensions-node/about/src/package_manager.rs` | 1 |
| `extensions-rust/cargo-toml/src/workspace_root.rs` | 1 |
| `extensions-rust/deps/src/parse/deny.rs` | 1 |
| `extensions-rust/metadata/src/views.rs` | 1 |
| `extensions-rust/test-coverage/src/tests.rs` | 1 |
| `extensions/duckdb/src/sql/ingest/sql.rs` | 1 |
| `extensions/duckdb/src/sql/query/helpers.rs` | 1 |
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every site listed in the scope table is either fixed or carries an `#[allow]` at the narrowest scope that works, with a comment giving the reason (docs/clippy.md layer 2 or 3)
- [ ] #2 The line(s) for `redundant_pub_crate` are deleted from the temporary-allow block in the root `Cargo.toml`, and the lint reaches the workspace at `deny`
- [ ] #3 `cargo clippy --workspace --all-features --all-targets -- -D warnings` passes
- [ ] #4 `cargo nextest run --workspace --all-features` and `cargo test --workspace --doc` pass
<!-- AC:END -->
