---
id: TASK-1679
title: 'CLIPPY: drop the workspace allow for `too_long_first_doc_paragraph` (nursery)'
status: Done
assignee:
  - TASK-1688
created_date: '2026-08-25 21:00'
updated_date: '2026-08-26 21:44'
labels:
  - code-review-rust
  - clippy
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Enabling `clippy::nursery` and the panic/arithmetic lints from the photo config surfaced 61 pre-existing sites for this lint family across 36 files. To keep `cargo clippy --workspace --all-features --all-targets -- -D warnings` green, the lint is currently allowed workspace-wide in the `# --- Temporary allows ---` block of the root `Cargo.toml`. That allow is the thing this task removes.

**Lint(s)**: `clippy::too_long_first_doc_paragraph`
**Sites**: 61 across 36 files

Doc comments whose first paragraph runs past the summary line, so rustdoc's item list shows a wall of text. Split the first sentence into its own paragraph. Documentation-only, no behaviour change.

## Scope

| File | Sites |
|---|---|
| `crates/core/src/text.rs` | 4 |
| `extensions-terraform/plan/src/lib.rs` | 4 |
| `extensions/about/src/manifest_cache.rs` | 4 |
| `crates/core/src/config/edit.rs` | 3 |
| `crates/core/src/style.rs` | 3 |
| `crates/core/src/subprocess/cap.rs` | 3 |
| `extensions-rust/cargo-toml/src/workspace_root.rs` | 3 |
| `extensions/duckdb/src/sql/validation.rs` | 3 |
| `crates/core/src/config/loader/global.rs` | 2 |
| `crates/core/src/test_utils.rs` | 2 |
| `extensions/about/src/identity.rs` | 2 |
| `extensions/about/src/providers.rs` | 2 |
| `extensions/about/src/workspace.rs` | 2 |
| `extensions/git/src/config.rs` | 2 |
| `crates/core/src/config/loader/mod.rs` | 1 |
| `crates/core/src/stack/mod.rs` | 1 |
| `crates/core/src/subprocess/mod.rs` | 1 |
| `crates/runner/src/command/mod.rs` | 1 |
| `crates/runner/src/display/render_config.rs` | 1 |
| `crates/theme/src/step_line_theme.rs` | 1 |
| `crates/theme/src/style/sgr.rs` | 1 |
| `extensions-rust/deps/src/lib.rs` | 1 |
| `extensions-rust/metadata/src/lib.rs` | 1 |
| `extensions-terraform/plan/src/render.rs` | 1 |
| `extensions/about/src/code.rs` | 1 |
| `extensions/about/src/manifest_io.rs` | 1 |
| `extensions/about/src/test_support.rs` | 1 |
| `extensions/about/src/text_util.rs` | 1 |
| `extensions/about/src/units.rs` | 1 |
| `extensions/config-checkers/src/json.rs` | 1 |
| `extensions/config-checkers/src/lib.rs` | 1 |
| `extensions/duckdb/src/schema.rs` | 1 |
| `extensions/duckdb/src/sql/ingest/sidecar.rs` | 1 |
| `extensions/git/src/provider.rs` | 1 |
| `extensions/hook-common/src/lib.rs` | 1 |
| `extensions/tokei/src/views.rs` | 1 |
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every site listed in the scope table is either fixed or carries an `#[allow]` at the narrowest scope that works, with a comment giving the reason (docs/clippy.md layer 2 or 3)
- [x] #2 The line(s) for `too_long_first_doc_paragraph` are deleted from the temporary-allow block in the root `Cargo.toml`, and the lint reaches the workspace at `deny`
- [x] #3 `cargo clippy --workspace --all-features --all-targets -- -D warnings` passes
- [x] #4 `cargo nextest run --workspace --all-features` and `cargo test --workspace --doc` pass
<!-- AC:END -->
