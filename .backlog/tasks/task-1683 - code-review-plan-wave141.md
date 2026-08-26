---
id: TASK-1683
title: code-review-plan-wave141
status: To Do
assignee:
  - code-review-wave
created_date: '2026-08-26 21:17'
updated_date: '2026-08-26 21:17'
labels:
  - code-review-wave
dependencies:
  - TASK-1672
  - TASK-1673
modified_files:
  - Cargo.toml
  - crates/cli/src/args.rs
  - crates/cli/src/extension_cmd.rs
  - crates/cli/src/help.rs
  - crates/cli/src/import_makefile_cmd.rs
  - crates/cli/src/theme_cmd.rs
  - crates/core/src/output.rs
  - crates/core/src/project_identity/card.rs
  - crates/core/src/project_identity/format.rs
  - crates/core/src/subprocess/drain.rs
  - crates/core/src/text.rs
  - crates/runner/src/command/events.rs
  - crates/runner/src/command/exec.rs
  - crates/runner/src/command/results.rs
  - crates/runner/src/command/secret_patterns.rs
  - crates/runner/src/display.rs
  - crates/runner/src/display/finalize.rs
  - extensions-go/about/src/go_mod.rs
  - extensions-go/about/src/go_syntax.rs
  - extensions-java/about/src/gradle/lexer.rs
  - extensions-java/about/src/maven/pom.rs
  - extensions-node/about/src/units.rs
  - extensions-rust/about/src/query.rs
  - extensions-rust/cargo-update/src/lib.rs
  - extensions-rust/deps/src/format.rs
  - extensions-rust/deps/src/parse/mod.rs
  - extensions-rust/deps/src/parse/upgrade.rs
  - extensions-rust/loc/src/counter.rs
  - extensions-rust/metadata/src/types.rs
  - extensions-rust/test-coverage/src/parse.rs
  - extensions-terraform/about/src/lib.rs
  - extensions/about/src/cards.rs
  - extensions/about/src/loc.rs
  - extensions/about/src/text_util.rs
  - extensions/about/src/workspace.rs
  - extensions/duckdb/src/sql/ingest/dir.rs
  - extensions/duckdb/src/sql/validation.rs
  - extensions/git/src/config.rs
  - extensions/git/src/remote.rs
  - extensions/hook-common/src/config.rs
  - extensions/text-fixers/src/binary.rs
  - extensions/text-fixers/src/eof.rs
  - extensions/text-fixers/src/trailing.rs
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave141
<!-- SECTION:DESCRIPTION:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Bounds- and char-boundary panics: `indexing_slicing` + `string_slice`. Same root cause and same fix idiom (`get`/`get_mut`/`split_at_checked`/`char_indices`), heavily co-located in the manifest parsers and display code.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-1684 [wave142] 35 files (Cargo.toml, crates/cli/src/help.rs...); TASK-1685 [wave143] 15 files (Cargo.toml, crates/cli/src/args.rs...); TASK-1686 [wave144] 24 files (Cargo.toml, crates/cli/src/args.rs...); TASK-1687 [wave145] 18 files (Cargo.toml, crates/cli/src/args.rs...); TASK-1688 [wave146] 6 files (Cargo.toml, crates/core/src/text.rs...); TASK-1689 [wave147] 2 files (Cargo.toml, extensions-rust/about/src/query.rs)

Every wave in this batch edits the `# --- Temporary allows ---` block in the root `Cargo.toml`, so a one-line merge there is expected on each landing.
<!-- SECTION:NOTES:END -->
