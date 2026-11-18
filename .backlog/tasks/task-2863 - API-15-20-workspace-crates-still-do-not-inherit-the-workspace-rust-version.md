---
id: TASK-2863
title: 'API-15: 20 workspace crates still do not inherit the workspace rust-version'
status: Triage
assignee: []
created_date: '2026-11-18 21:12'
labels:
  - code-review-rust
  - api
dependencies: []
modified_files:
  - extensions/about/Cargo.toml
  - crates/backlog/Cargo.toml
  - extensions/tokei/Cargo.toml
  - extensions-java/about/Cargo.toml
  - extensions/hook-common/Cargo.toml
  - extensions/sqlite/Cargo.toml
  - extensions-rust/metadata/Cargo.toml
  - extensions-rust/about/Cargo.toml
  - crates/core/Cargo.toml
  - extensions/create-coderabbit-tasks/Cargo.toml
  - extensions/create-review-tasks/Cargo.toml
  - extensions/run-before-commit/Cargo.toml
  - extensions-rust/cargo-update/Cargo.toml
  - extensions-node/about/Cargo.toml
  - crates/runner/Cargo.toml
  - extensions-terraform/about/Cargo.toml
  - extensions-rust/create-review-tasks/Cargo.toml
  - extensions-rust/deps/Cargo.toml
  - extensions-rust/foundation/Cargo.toml
  - extensions-rust/test-coverage/Cargo.toml
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: 20 member manifests (see modified files), `[package]` table

**What**: After TASK-2847 fixed ops-about-go and ops-config-checkers, these member manifests still omit `rust-version.workspace = true`, so they declare no MSRV although root `Cargo.toml` sets `rust-version = "1.97"`: crates/core, crates/backlog, crates/runner, extensions/{about,tokei,hook-common,sqlite,create-coderabbit-tasks,create-review-tasks,run-before-commit}, extensions-rust/{metadata,about,cargo-update,create-review-tasks,deps,foundation,test-coverage}, extensions-java/about, extensions-node/about, extensions-terraform/about.

**Why it matters**: The workspace MSRV contract is not attached to these crates, so cargo cannot reject an older toolchain clearly and MSRV-gated rules have nothing to check against. The earlier review filed per-crate tasks for only two of them.

**Origin**: discovered during TASK-2847 while fixing TASK-2722 / TASK-2780.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every workspace member manifest declares rust-version.workspace = true
- [ ] #2 cargo metadata reports rust_version 1.97 for every workspace package
<!-- AC:END -->
