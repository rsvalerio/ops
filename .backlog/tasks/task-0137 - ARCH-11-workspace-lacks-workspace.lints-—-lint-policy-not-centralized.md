---
id: TASK-0137
title: 'ARCH-11: workspace lacks [workspace.lints] — lint policy not centralized'
status: In Progress
assignee: []
created_date: '2026-04-22 21:16'
updated_date: '2026-04-23 14:32'
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

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Deferred: adding [workspace.lints] with clippy::pedantic warn triggers dozens of new warnings across ~25 crates and  would fail the verify gate. Proper scope is: (1) add [workspace.lints] scaffolding with only clippy::all + rust warnings, (2) enable pedantic as a separate PR with systematic fixes (probably a wave of its own). Leaving task In Progress so next wave picks it up with dedicated scope rather than smuggling a huge diff into this one.
<!-- SECTION:NOTES:END -->
