---
id: TASK-0633
title: >-
  ARCH-11: serial_test pinned directly in three hook crates instead of
  workspace.dependencies
status: Done
assignee:
  - TASK-0637
created_date: '2026-04-29 05:50'
updated_date: '2026-04-29 06:33'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/Cargo.toml:16`, `extensions/run-before-push/Cargo.toml:17`, `extensions/run-before-commit/Cargo.toml:17`

**What**: These three crates declare `serial_test = "3"` directly under `[dev-dependencies]`. The workspace root already defines `serial_test = "3"` in `[workspace.dependencies]` (Cargo.toml:61), and the convention everywhere else in the workspace is `serial_test = { workspace = true }`.

**Why it matters**: ARCH-11 — diverging dependency declarations defeat workspace pinning, create drift on version bumps, and force every refresh to touch N Cargo.toml files instead of one. A future bump to `serial_test = "4"` in workspace will silently leave these three crates on v3.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 All three crates use `serial_test = { workspace = true }` in dev-dependencies
- [ ] #2 cargo build --all-targets and cargo test pass
- [ ] #3 No other workspace-managed dep is directly pinned in these three Cargo.toml files
<!-- AC:END -->
