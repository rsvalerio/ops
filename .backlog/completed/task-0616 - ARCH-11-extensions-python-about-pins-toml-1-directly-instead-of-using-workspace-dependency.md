---
id: TASK-0616
title: >-
  ARCH-11: extensions-python/about pins toml = "1" directly instead of using
  workspace dependency
status: Done
assignee:
  - TASK-0637
created_date: '2026-04-29 05:20'
updated_date: '2026-04-29 06:33'
labels:
  - code-review-rust
  - ARCH
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/Cargo.toml:18`

**What**: extensions-python/about/Cargo.toml declares `toml = "1"` per-crate even though workspace root (Cargo.toml:52) exposes `toml = "1"` and other crates use `{ workspace = true }`. Same pattern repeats in extensions-rust/tools/Cargo.toml:17, extensions-rust/cargo-toml/Cargo.toml:14, extensions-rust/about/Cargo.toml:20. TASK-0413 centralised tempfile/sha2/hex/tokei/duckdb/toml_edit but toml was missed.

**Why it matters**: ARCH-11 — per-crate version pins for workspace-wide dep is the divergence risk: bumping requires touching N manifests; build can drift to mismatched toml resolutions.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Workspace Cargo.toml adds toml to [workspace.dependencies]
- [ ] #2 All four crates reference toml = { workspace = true }
- [ ] #3 cargo build --workspace succeeds
<!-- AC:END -->
