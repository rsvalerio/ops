---
id: TASK-2886
title: 'API-15: nothing gates rust-version inheritance, so new member crates drop the workspace MSRV again'
status: Triage
assignee: []
created_date: '2026-11-19 08:08'
labels:
  - code-review-rust
  - api
dependencies: []
modified_files:
  - crates/cli/src/workspace_check_cmd.rs
  - .ops.toml
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/workspace_check_cmd.rs`, member `Cargo.toml` files

**What**: Every member now declares `rust-version.workspace = true`, but no gate checks it. The gap has been fixed piecemeal three times (two single-crate tasks, then a 20-crate sweep), each found only by review. A new crate created without the key passes `ops verify` and CI.

**Why it matters**: The MSRV contract silently detaches from new crates, and `cargo` cannot reject an older toolchain for them.

**Origin**: discovered during TASK-2882 while fixing TASK-2863.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A gate run by ops verify or CI fails when a workspace member's cargo metadata rust_version differs from the root rust-version
<!-- AC:END -->
