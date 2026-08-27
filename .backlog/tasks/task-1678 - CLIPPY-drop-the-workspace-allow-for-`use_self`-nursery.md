---
id: TASK-1678
title: 'CLIPPY: drop the workspace allow for `use_self` (nursery)'
status: Done
assignee:
  - TASK-1686
created_date: '2026-08-25 21:00'
updated_date: '2026-08-26 22:22'
labels:
  - code-review-rust
  - clippy
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Enabling `clippy::nursery` and the panic/arithmetic lints from the photo config surfaced 87 pre-existing sites for this lint family across 22 files. To keep `cargo clippy --workspace --all-features --all-targets -- -D warnings` green, the lint is currently allowed workspace-wide in the `# --- Temporary allows ---` block of the root `Cargo.toml`. That allow is the thing this task removes.

**Lint(s)**: `clippy::use_self`
**Sites**: 87 across 22 files

`TypeName` written where `Self` applies, inside `impl` blocks. Mechanical and `clippy --fix` handles it — but per AGENTS.md, read the diff: `--fix` has silently deleted load-bearing code in this repo before.

## Scope

| File | Sites |
|---|---|
| `extensions-terraform/plan/src/model.rs` | 29 |
| `crates/cli/src/sec_cmd.rs` | 13 |
| `extensions-rust/cargo-toml/src/types.rs` | 11 |
| `crates/core/src/subprocess/mod.rs` | 9 |
| `crates/core/src/config/commands.rs` | 4 |
| `extensions-rust/metadata/src/types.rs` | 4 |
| `crates/runner/src/display/render_config.rs` | 2 |
| `crates/cli/src/test_utils.rs` | 1 |
| `crates/core/src/stack/mod.rs` | 1 |
| `crates/core/src/test_utils.rs` | 1 |
| `crates/extension/src/tests.rs` | 1 |
| `crates/runner/src/command/tests/expand.rs` | 1 |
| `crates/runner/src/command/tests/parallel.rs` | 1 |
| `extensions-rust/cargo-update/src/tests.rs` | 1 |
| `extensions-rust/deps/src/test_support.rs` | 1 |
| `extensions-terraform/about/src/lib.rs` | 1 |
| `extensions/about/src/test_support.rs` | 1 |
| `extensions/duckdb/src/error.rs` | 1 |
| `extensions/duckdb/src/sql/ingest/orchestrator.rs` | 1 |
| `extensions/git/src/config.rs` | 1 |
| `extensions/hook-common/src/git.rs` | 1 |
| `extensions/run-before-commit/src/lib.rs` | 1 |
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every site listed in the scope table is either fixed or carries an `#[allow]` at the narrowest scope that works, with a comment giving the reason (docs/clippy.md layer 2 or 3)
- [x] #2 The line(s) for `use_self` are deleted from the temporary-allow block in the root `Cargo.toml`, and the lint reaches the workspace at `deny`
- [x] #3 `cargo clippy --workspace --all-features --all-targets -- -D warnings` passes
- [x] #4 `cargo nextest run --workspace --all-features` and `cargo test --workspace --doc` pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed under TASK-1686 (wave144), branch code-review/TASK-1686.

87 sites cleared, matching the scope table. `cargo clippy --fix` was NOT used: per AGENTS.md and docs/clippy.md it has silently deleted load-bearing code in this repo. Instead the machine-applicable spans were extracted from clippy's JSON output and applied directly, then the whole diff was audited token-by-token - every changed pair differs only by an identifier replaced with `Self`, with no other edits. `ops extension list` still lists `git` and `text-fixers`, confirming the `extern crate` lines in `crates/cli/src/main.rs` are intact.

One case worth recording: in `extensions-rust/metadata/src/types.rs` the impl is `impl JsonValueExt for serde_json::Value`, so `serde_json::Value::as_str` became `Self::as_str`. That is correct - `Self` is `serde_json::Value` there - and resolves to the same inherent method.

No `#[allow]` was needed anywhere.

Verified: `ops verify` clean, `cargo nextest run --workspace --all-features` 2405 passed, `cargo test --workspace --doc` clean.
<!-- SECTION:NOTES:END -->
