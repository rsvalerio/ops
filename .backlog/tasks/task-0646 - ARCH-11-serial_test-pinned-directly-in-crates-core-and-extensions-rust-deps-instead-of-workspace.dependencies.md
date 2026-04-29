---
id: TASK-0646
title: >-
  ARCH-11: serial_test pinned directly in crates/core and extensions-rust/deps
  instead of workspace.dependencies
status: To Do
assignee: []
created_date: '2026-04-29 09:02'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Files**: `crates/core/Cargo.toml:29`, `extensions-rust/deps/Cargo.toml:20`

**What**: These two crates declare `serial_test = "3"` directly under `[dev-dependencies]`. The workspace root already defines `serial_test = "3"` in `[workspace.dependencies]` (Cargo.toml:61), and the convention everywhere else in the workspace is `serial_test = { workspace = true }`.

**Why it matters**: ARCH-11 — diverging dependency declarations defeat workspace pinning, create drift on version bumps, and force every refresh to touch N Cargo.toml files instead of one. A future bump to `serial_test = "4"` in workspace will silently leave these two crates on v3.

**Context**: TASK-0633 (Wave 42) covered the three hook crates (extensions/{hook-common,run-before-commit,run-before-push}) but the same pattern was found in core and deps and was out of scope.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 All two crates use serial_test = { workspace = true } in dev-dependencies
- [ ] #2 cargo build --all-targets and cargo test pass
- [ ] #3 No other workspace-managed dep is directly pinned in these two Cargo.toml files
<!-- AC:END -->
