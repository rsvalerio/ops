---
id: TASK-0413
title: >-
  ARCH-11: extensions crates pin shared deps (tempfile, sha2, hex, tokei,
  duckdb, toml_edit) per-crate instead of workspace.dependencies
status: To Do
assignee:
  - TASK-0417
created_date: '2026-04-26 09:54'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/Cargo.toml`, `extensions/duckdb/Cargo.toml`, `extensions/git/Cargo.toml`, `extensions/hook-common/Cargo.toml`, `extensions/run-before-commit/Cargo.toml`, `extensions/run-before-push/Cargo.toml`, `extensions/tokei/Cargo.toml`

**What**: All seven extension crates declare `tempfile = "3"` directly; `duckdb = { version = "1.10502", features = ["bundled"] }` is repeated in `extensions/duckdb` and elsewhere; `sha2 = "0.10"`, `hex = "0.4"`, `tokei = "14.0"`, `toml_edit = "0.25"` are pinned per-crate. The workspace already centralises versions for `ops-core`, `ops-extension`, `linkme`, `serde_json`, `anyhow`, `tracing`, `thiserror`, `serde` via `workspace = true`, so the pattern is established — these external deps were just missed.

**Why it matters**: ARCH-11 — version drift across siblings becomes possible with each upgrade PR (today they happen to match, but nothing enforces it), and a CVE bump on `tokei` / `duckdb` / `sha2` becomes a multi-file change instead of a single `[workspace.dependencies]` edit. Promoting these to `[workspace.dependencies]` and switching call sites to `dep = { workspace = true }` fixes both.

<!-- scan confidence: high; verified by grep across extensions/*/Cargo.toml -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 tempfile, sha2, hex, tokei, duckdb, toml_edit moved to [workspace.dependencies]
- [ ] #2 every extension crate Cargo.toml uses { workspace = true } (with feature opt-in only) for these deps
- [ ] #3 no per-crate version literal remains for the listed dependencies
<!-- AC:END -->
