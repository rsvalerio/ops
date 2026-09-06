---
id: TASK-1680
title: 'CLIPPY: drop the workspace allow for `option_if_let_else` (nursery)'
status: Done
assignee:
  - TASK-1687
created_date: '2026-08-25 21:00'
updated_date: '2026-08-26 21:51'
labels:
  - code-review-rust
  - clippy
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Enabling `clippy::nursery` and the panic/arithmetic lints from the photo config surfaced 43 pre-existing sites for this lint family across 35 files. To keep `cargo clippy --workspace --all-features --all-targets -- -D warnings` green, the lint is currently allowed workspace-wide in the `# --- Temporary allows ---` block of the root `Cargo.toml`. That allow is the thing this task removes.

**Lint(s)**: `clippy::option_if_let_else`
**Sites**: 43 across 35 files

`if let Some(x) = .. { .. } else { .. }` that reads better as `map_or`/`map_or_else`. Judgement call per site — this lint is in nursery precisely because the suggestion is sometimes less readable than the original, especially when the branches are long or borrow. Fix where it genuinely reads better; `#[allow]` with a reason where it does not.

## Scope

| File | Sites |
|---|---|
| `crates/core/src/text.rs` | 3 |
| `crates/cli/src/registry/discovery.rs` | 2 |
| `crates/runner/src/command/exec.rs` | 2 |
| `extensions-go/about/src/modules.rs` | 2 |
| `extensions-rust/about/src/units.rs` | 2 |
| `extensions/about/src/cards.rs` | 2 |
| `extensions/about/src/workspace.rs` | 2 |
| `crates/cli/src/about_cmd.rs` | 1 |
| `crates/cli/src/args.rs` | 1 |
| `crates/cli/src/help.rs` | 1 |
| `crates/cli/src/registry/registration.rs` | 1 |
| `crates/cli/src/subcommands.rs` | 1 |
| `crates/core/src/expand.rs` | 1 |
| `crates/core/src/project_identity/card.rs` | 1 |
| `crates/core/src/stack/mod.rs` | 1 |
| `crates/core/src/subprocess/cap.rs` | 1 |
| `crates/runner/src/command/mod.rs` | 1 |
| `crates/runner/src/command/results.rs` | 1 |
| `crates/runner/src/display.rs` | 1 |
| `crates/runner/src/display/finalize.rs` | 1 |
| `crates/theme/src/style/sgr.rs` | 1 |
| `extensions-java/about/src/gradle/lexer.rs` | 1 |
| `extensions-rust/cargo-toml/src/lib.rs` | 1 |
| `extensions-rust/cargo-update/src/lib.rs` | 1 |
| `extensions-rust/metadata/src/types.rs` | 1 |
| `extensions-rust/test-coverage/src/parse.rs` | 1 |
| `extensions-rust/test-coverage/src/subprocess.rs` | 1 |
| `extensions-terraform/plan/src/lib.rs` | 1 |
| `extensions/duckdb/src/connection.rs` | 1 |
| `extensions/duckdb/src/sql/ingest/sql.rs` | 1 |
| `extensions/duckdb/src/sql/query/helpers.rs` | 1 |
| `extensions/git/src/config.rs` | 1 |
| `extensions/git/src/remote.rs` | 1 |
| `extensions/hook-common/src/paths.rs` | 1 |
| `extensions/text-fixers/src/trailing.rs` | 1 |
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every site listed in the scope table is either fixed or carries an `#[allow]` at the narrowest scope that works, with a comment giving the reason (docs/clippy.md layer 2 or 3)
- [x] #2 The line(s) for `option_if_let_else` are deleted from the temporary-allow block in the root `Cargo.toml`, and the lint reaches the workspace at `deny`
- [x] #3 `cargo clippy --workspace --all-features --all-targets -- -D warnings` passes
- [x] #4 `cargo nextest run --workspace --all-features` and `cargo test --workspace --doc` pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Cleared under TASK-1687 (wave145) on branch code-review/TASK-1687.

All 43 sites addressed: 36 converted, 7 site-local `#[allow(clippy::option_if_let_else)]`
(covering 8 sites) where `map_or`/`map_or_else` genuinely reads worse. Each allow carries
a reason comment per docs/clippy.md layer 3:

- `crates/cli/src/registry/registration.rs:134` — borrowck: the `Some` closure would hold
  the `&mut` from `get_mut` while the `None` closure needs a second `&mut owners`.
- `crates/core/src/text.rs:61` and `crates/runner/src/command/results.rs:152` — the `Some`
  arm is a multi-arm `parse` classification matrix; a closure would hoist the trivial
  default ahead of the logic and nest the matrix one level deeper.
- `crates/runner/src/command/exec.rs:57` — async: both arms move and `.await` the same
  future; a non-async closure cannot await it.
- `extensions/about/src/workspace.rs:120` (one allow also covers the nested site at :140),
  `extensions-go/about/src/modules.rs:36`, `extensions-rust/about/src/units.rs:129` —
  multi-statement branches that log distinct breadcrumbs before yielding, inside already
  deeply indented bodies.

Conversions preferred `is_none_or` / `is_some_and` / `is_ok_and` over `map_or(true/false, ..)`
so the change does not trade this lint for `clippy::unnecessary_map_or`, and
`unwrap_or_else` over `map_or_else(.., |v| v)` in extensions-rust/metadata/src/types.rs.

`option_if_let_else` line deleted from the `# --- Temporary allows ---` block in the root
Cargo.toml; the lint now reaches the workspace at deny via `nursery`.

Gates: `ops verify` clean; `cargo nextest run --workspace --all-features` 2405 passed,
7 skipped; `cargo test --workspace --doc` clean.
<!-- SECTION:NOTES:END -->
