---
id: TASK-1674
title: 'CLIPPY: drop the workspace allow for `as_conversions`'
status: Done
assignee:
  - TASK-1684
created_date: '2026-08-25 21:00'
updated_date: '2026-08-26 22:39'
labels:
  - code-review-rust
  - clippy
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Enabling `clippy::nursery` and the panic/arithmetic lints from the photo config surfaced 61 pre-existing sites for this lint family across 29 files. To keep `cargo clippy --workspace --all-features --all-targets -- -D warnings` green, the lint is currently allowed workspace-wide in the `# --- Temporary allows ---` block of the root `Cargo.toml`. That allow is the thing this task removes.

**Lint(s)**: `clippy::as_conversions`
**Sites**: 61 across 29 files

`as` casts that can silently truncate or change sign. Replace with `TryFrom`/`try_into` plus an error path, or `From` where the conversion is total. Where a cast is provably lossless, keep it with an `#[allow]` and the reason.

## Scope

| File | Sites |
|---|---|
| `crates/theme/src/configurable.rs` | 7 |
| `crates/core/src/subprocess/drain.rs` | 6 |
| `extensions/about/src/manifest_io.rs` | 4 |
| `extensions/git/src/config.rs` | 4 |
| `crates/runner/src/command/results.rs` | 3 |
| `extensions/duckdb/src/sql/ingest/dir.rs` | 3 |
| `crates/core/src/config/commands.rs` | 2 |
| `crates/core/src/subprocess/cap.rs` | 2 |
| `crates/core/src/ui.rs` | 2 |
| `crates/runner/src/command/tests/exec.rs` | 2 |
| `crates/runner/src/command/tests/sequential.rs` | 2 |
| `crates/theme/src/step_line_theme.rs` | 2 |
| `crates/theme/src/tests/format_duration.rs` | 2 |
| `extensions-terraform/plan/src/lib.rs` | 2 |
| `extensions/about/src/loc.rs` | 2 |
| `extensions/duckdb/src/schema.rs` | 2 |
| `extensions/duckdb/src/sql/ingest/sidecar.rs` | 2 |
| `crates/core/src/config/edit.rs` | 1 |
| `crates/core/src/config/loader/mod.rs` | 1 |
| `crates/core/src/stack/metadata.rs` | 1 |
| `crates/core/src/text.rs` | 1 |
| `crates/runner/src/command/exec.rs` | 1 |
| `crates/runner/src/command/tests/parallel.rs` | 1 |
| `crates/theme/src/tests/boxed_layout.rs` | 1 |
| `crates/theme/src/tests/render_report.rs` | 1 |
| `extensions-java/about/src/gradle/lexer.rs` | 1 |
| `extensions-node/about/src/units.rs` | 1 |
| `extensions-rust/cargo-update/src/lib.rs` | 1 |
| `extensions-terraform/plan/src/render.rs` | 1 |
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every site listed in the scope table is either fixed or carries an `#[allow]` at the narrowest scope that works, with a comment giving the reason (docs/clippy.md layer 2 or 3)
- [x] #2 The line(s) for `as_conversions` are deleted from the temporary-allow block in the root `Cargo.toml`, and the lint reaches the workspace at `deny`
- [x] #3 `cargo clippy --workspace --all-features --all-targets -- -D warnings` passes
- [x] #4 `cargo nextest run --workspace --all-features` and `cargo test --workspace --doc` pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Cleared in wave142 (TASK-1684). All 61 sites fixed. Most resolved to a checked conversion — `From` where the widening is total (`u32::from(char)`, `usize::from(u16)`, `char::from(u8)`), `TryFrom` with a domain-correct `unwrap_or` fallback for the size-cap comparisons, and several casts removed outright (`ptr.addr()`, `.as_slice()`, `&[]` unsize casts, and `(b'a'..=b'z').cycle()` fixture generators). Four site-local `#[allow]`s remain, each statement-scoped with a proof comment: one in extensions/about/src/loc.rs (i64 -> f64 percentage, extends an existing cast_precision_loss allow), one in crates/theme/src/step_line_theme.rs and two in crates/theme/src/tests/format_duration.rs (u64 <-> f64 clamp bound, which no From/TryFrom pair expresses). `as_conversions` is a restriction-group lint, so it was moved up to the explicit deny list rather than merely deleted from the temporary-allow block.
<!-- SECTION:NOTES:END -->
