---
id: TASK-1672
title: 'CLIPPY: drop the workspace allow for `indexing_slicing`'
status: To Do
assignee:
  - TASK-1683
created_date: '2026-08-25 21:00'
updated_date: '2026-08-26 21:17'
labels:
  - code-review-rust
  - clippy
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Enabling `clippy::nursery` and the panic/arithmetic lints from the photo config surfaced 69 pre-existing sites for this lint family across 25 files. To keep `cargo clippy --workspace --all-features --all-targets -- -D warnings` green, the lint is currently allowed workspace-wide in the `# --- Temporary allows ---` block of the root `Cargo.toml`. That allow is the thing this task removes.

**Lint(s)**: `clippy::indexing_slicing`
**Sites**: 69 across 25 files

`a[i]` and `&s[a..b]` in production code, each a potential panic. Replace with `get`/`get_mut`/`get(..)` plus an error or a documented fallback. Test code is already exempt via `allow-indexing-slicing-in-tests` in clippy.toml, so every site here is production.

## Scope

| File | Sites |
|---|---|
| `crates/runner/src/display.rs` | 9 |
| `crates/core/src/output.rs` | 7 |
| `crates/runner/src/command/secret_patterns.rs` | 5 |
| `crates/runner/src/display/finalize.rs` | 5 |
| `extensions-go/about/src/go_mod.rs` | 4 |
| `extensions/git/src/remote.rs` | 4 |
| `extensions/text-fixers/src/eof.rs` | 4 |
| `extensions/text-fixers/src/trailing.rs` | 4 |
| `extensions-go/about/src/go_syntax.rs` | 3 |
| `crates/core/src/subprocess/drain.rs` | 2 |
| `crates/runner/src/command/exec.rs` | 2 |
| `extensions-rust/deps/src/parse/upgrade.rs` | 2 |
| `extensions-rust/loc/src/counter.rs` | 2 |
| `extensions-rust/test-coverage/src/parse.rs` | 2 |
| `extensions-terraform/about/src/lib.rs` | 2 |
| `extensions/duckdb/src/sql/validation.rs` | 2 |
| `extensions/hook-common/src/config.rs` | 2 |
| `crates/cli/src/args.rs` | 1 |
| `crates/cli/src/extension_cmd.rs` | 1 |
| `extensions-java/about/src/gradle/lexer.rs` | 1 |
| `extensions-rust/deps/src/format.rs` | 1 |
| `extensions-rust/metadata/src/types.rs` | 1 |
| `extensions/about/src/loc.rs` | 1 |
| `extensions/duckdb/src/sql/ingest/dir.rs` | 1 |
| `extensions/text-fixers/src/binary.rs` | 1 |
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every site listed in the scope table is either fixed or carries an `#[allow]` at the narrowest scope that works, with a comment giving the reason (docs/clippy.md layer 2 or 3)
- [ ] #2 The line(s) for `indexing_slicing` are deleted from the temporary-allow block in the root `Cargo.toml`, and the lint reaches the workspace at `deny`
- [ ] #3 `cargo clippy --workspace --all-features --all-targets -- -D warnings` passes
- [ ] #4 `cargo nextest run --workspace --all-features` and `cargo test --workspace --doc` pass
<!-- AC:END -->
