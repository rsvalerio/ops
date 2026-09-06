---
id: TASK-1688
title: code-review-plan-wave146
status: Done
assignee:
  - code-review-wave
created_date: '2026-08-26 21:17'
updated_date: '2026-08-26 21:45'
labels:
  - code-review-wave
dependencies:
  - TASK-1679
modified_files:
  - Cargo.toml
  - crates/core/src/config/edit.rs
  - crates/core/src/config/loader/global.rs
  - crates/core/src/config/loader/mod.rs
  - crates/core/src/stack/mod.rs
  - crates/core/src/style.rs
  - crates/core/src/subprocess/cap.rs
  - crates/core/src/subprocess/mod.rs
  - crates/core/src/test_utils.rs
  - crates/core/src/text.rs
  - crates/runner/src/command/mod.rs
  - crates/runner/src/display/render_config.rs
  - crates/theme/src/step_line_theme.rs
  - crates/theme/src/style/sgr.rs
  - extensions-rust/cargo-toml/src/workspace_root.rs
  - extensions-rust/deps/src/lib.rs
  - extensions-rust/metadata/src/lib.rs
  - extensions-terraform/plan/src/lib.rs
  - extensions-terraform/plan/src/render.rs
  - extensions/about/src/code.rs
  - extensions/about/src/identity.rs
  - extensions/about/src/manifest_cache.rs
  - extensions/about/src/manifest_io.rs
  - extensions/about/src/providers.rs
  - extensions/about/src/test_support.rs
  - extensions/about/src/text_util.rs
  - extensions/about/src/units.rs
  - extensions/about/src/workspace.rs
  - extensions/config-checkers/src/json.rs
  - extensions/config-checkers/src/lib.rs
  - extensions/duckdb/src/schema.rs
  - extensions/duckdb/src/sql/ingest/sidecar.rs
  - extensions/duckdb/src/sql/validation.rs
  - extensions/git/src/config.rs
  - extensions/git/src/provider.rs
  - extensions/hook-common/src/lib.rs
  - extensions/tokei/src/views.rs
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave146
<!-- SECTION:DESCRIPTION:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
`too_long_first_doc_paragraph` alone: doc-comment-only, zero behaviour change, so it can land independently of every code-changing wave.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-1683 [wave141] 6 files (Cargo.toml, crates/core/src/text.rs...); TASK-1684 [wave142] 17 files (Cargo.toml, crates/core/src/config/edit.rs...); TASK-1685 [wave143] 7 files (Cargo.toml, crates/core/src/config/loader/global.rs...); TASK-1686 [wave144] 14 files (Cargo.toml, crates/core/src/config/edit.rs...); TASK-1687 [wave145] 9 files (Cargo.toml, crates/core/src/stack/mod.rs...); TASK-1689 [wave147] 3 files (Cargo.toml, extensions/about/src/manifest_cache.rs...)

Every wave in this batch edits the `# --- Temporary allows ---` block in the root `Cargo.toml`, so a one-line merge there is expected on each landing.

Branch: code-review/TASK-1688
<!-- SECTION:NOTES:END -->
