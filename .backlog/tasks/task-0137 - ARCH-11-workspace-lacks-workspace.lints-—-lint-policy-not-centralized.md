---
id: TASK-0137
title: 'ARCH-11: workspace lacks [workspace.lints] — lint policy not centralized'
status: To Do
assignee: []
created_date: '2026-04-22 21:16'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - arch
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `Cargo.toml` (workspace root)

**What**: The workspace defines [workspace.dependencies] for version alignment but not [workspace.lints]. Individual crates can (and do) drift in clippy/rustc lint level.

**Why it matters**: Inconsistent lint enforcement lets warnings slip in for some crates while others are strict; ARCH-11 calls for unified lint policy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add [workspace.lints] with agreed clippy/rustc categories and levels (at minimum clippy::pedantic warn, clippy::unwrap_used in non-test)
- [ ] #2 Each crate/extension Cargo.toml sets [lints] workspace = true
<!-- AC:END -->
