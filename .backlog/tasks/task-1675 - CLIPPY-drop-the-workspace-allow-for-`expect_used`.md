---
id: TASK-1675
title: 'CLIPPY: drop the workspace allow for `expect_used`'
status: Done
assignee:
  - TASK-1685
created_date: '2026-08-25 21:00'
updated_date: '2026-08-26 21:52'
labels:
  - code-review-rust
  - clippy
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Enabling `clippy::nursery` and the panic/arithmetic lints from the photo config surfaced 22 pre-existing sites for this lint family across 14 files. To keep `cargo clippy --workspace --all-features --all-targets -- -D warnings` green, the lint is currently allowed workspace-wide in the `# --- Temporary allows ---` block of the root `Cargo.toml`. That allow is the thing this task removes.

**Lint(s)**: `clippy::expect_used`
**Sites**: 22 across 14 files

`.expect()` in production code. `allow-expect-in-tests` already exempts test code, so these are real panic sites in the CLI and core config loader. Convert to `?` with a typed error, or keep with an `#[allow]` and a comment proving the invariant.

## Scope

| File | Sites |
|---|---|
| `crates/cli/tests/integration.rs` | 7 |
| `crates/core/src/config/loader/global.rs` | 3 |
| `crates/cli/src/args.rs` | 1 |
| `crates/cli/src/extension_cmd.rs` | 1 |
| `crates/cli/src/import_makefile_cmd.rs` | 1 |
| `crates/cli/src/main.rs` | 1 |
| `crates/cli/src/theme_cmd.rs` | 1 |
| `crates/core/src/stack/mod.rs` | 1 |
| `crates/extension/src/data.rs` | 1 |
| `crates/runner/src/display/style.rs` | 1 |
| `extensions/about/src/workspace.rs` | 1 |
| `extensions/duckdb/src/sql/query/coverage.rs` | 1 |
| `extensions/git/src/config.rs` | 1 |
| `extensions/hook-common/src/git_state.rs` | 1 |
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every site listed in the scope table is either fixed or carries an `#[allow]` at the narrowest scope that works, with a comment giving the reason (docs/clippy.md layer 2 or 3)
- [x] #2 The line(s) for `expect_used` are deleted from the temporary-allow block in the root `Cargo.toml`, and the lint reaches the workspace at `deny`
- [x] #3 `cargo clippy --workspace --all-features --all-targets -- -D warnings` passes
- [x] #4 `cargo nextest run --workspace --all-features` and `cargo test --workspace --doc` pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
TASK-1685 (wave143): 22 expect_used sites cleared — real fixes where a total form existed (git config UTF-8 decode via `String::from_utf8` match, `matches_exclude` via `split_once`, `preprocess_args` via `Vec::remove`, `spawn_drain`/`in_flight` fallbacks, infallible `write!` into String), narrow `#[allow(clippy::expect_used)]` with a documented invariant elsewhere (poisoned locks, statically-valid templates/aliases/directives, cache-by-construction lookups). `crates/cli/tests/integration.rs` got a file-level allow (docs/clippy.md layer 2) because an integration-test target is its own crate and its helpers sit outside `#[test]` bodies, so `allow-expect-in-tests` does not reach them; docs/clippy.md updated to say so. `expect_used = "allow"` deleted from the root Cargo.toml temporary-allow block.
<!-- SECTION:NOTES:END -->
