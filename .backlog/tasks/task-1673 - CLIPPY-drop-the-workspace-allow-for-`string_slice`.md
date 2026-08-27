---
id: TASK-1673
title: 'CLIPPY: drop the workspace allow for `string_slice`'
status: Done
assignee:
  - TASK-1683
created_date: '2026-08-25 21:00'
updated_date: '2026-08-26 22:36'
labels:
  - code-review-rust
  - clippy
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Enabling `clippy::nursery` and the panic/arithmetic lints from the photo config surfaced 66 pre-existing sites for this lint family across 24 files. To keep `cargo clippy --workspace --all-features --all-targets -- -D warnings` green, the lint is currently allowed workspace-wide in the `# --- Temporary allows ---` block of the root `Cargo.toml`. That allow is the thing this task removes.

**Lint(s)**: `clippy::string_slice`
**Sites**: 66 across 24 files

Byte-range slicing of `str`, which panics when the index is not a UTF-8 char boundary. Concentrated in the Java/Terraform manifest parsers and the CLI help renderer, all of which handle user-supplied text. Use `char_indices`, `split_at_checked`, `get(..)`, or the existing `unicode-width` helpers.

## Scope

| File | Sites |
|---|---|
| `extensions-java/about/src/maven/pom.rs` | 12 |
| `extensions-java/about/src/gradle/lexer.rs` | 8 |
| `crates/cli/src/help.rs` | 6 |
| `extensions-terraform/about/src/lib.rs` | 5 |
| `extensions/about/src/workspace.rs` | 4 |
| `extensions/git/src/config.rs` | 4 |
| `extensions-node/about/src/units.rs` | 3 |
| `extensions/git/src/remote.rs` | 3 |
| `crates/cli/src/import_makefile_cmd.rs` | 2 |
| `crates/cli/src/theme_cmd.rs` | 2 |
| `crates/core/src/text.rs` | 2 |
| `extensions-rust/about/src/query.rs` | 2 |
| `extensions-rust/deps/src/parse/upgrade.rs` | 2 |
| `crates/core/src/project_identity/card.rs` | 1 |
| `crates/core/src/project_identity/format.rs` | 1 |
| `crates/runner/src/command/events.rs` | 1 |
| `crates/runner/src/command/results.rs` | 1 |
| `crates/runner/src/command/secret_patterns.rs` | 1 |
| `extensions-go/about/src/go_mod.rs` | 1 |
| `extensions-go/about/src/go_syntax.rs` | 1 |
| `extensions-rust/cargo-update/src/lib.rs` | 1 |
| `extensions-rust/deps/src/parse/mod.rs` | 1 |
| `extensions/about/src/cards.rs` | 1 |
| `extensions/about/src/text_util.rs` | 1 |
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every site listed in the scope table is either fixed or carries an `#[allow]` at the narrowest scope that works, with a comment giving the reason (docs/clippy.md layer 2 or 3)
- [x] #2 The line(s) for `string_slice` are deleted from the temporary-allow block in the root `Cargo.toml`, and the lint reaches the workspace at `deny`
- [x] #3 `cargo clippy --workspace --all-features --all-targets -- -D warnings` passes
- [x] #4 `cargo nextest run --workspace --all-features` and `cargo test --workspace --doc` pass
<!-- AC:END -->
