---
id: TASK-1612
title: 'ARCH-11: test-coverage Cargo.toml does not inherit workspace lints'
status: To Do
assignee:
  - TASK-1635
created_date: '2026-05-22 06:49'
updated_date: '2026-05-22 10:17'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/Cargo.toml:1-23`

**What**: The manifest inherits version, edition, license, and dependency versions from the workspace but does not declare `[lints] workspace = true`. No sibling `extensions-rust/*` crate inherits lints either — this is a workspace-wide gap.

**Why it matters**: ARCH-11 — shared lint config prevents drift in clippy/rustc warning policy across member crates. Once `[workspace.lints]` is established (or if one exists and isn't consumed), this crate silently misses new rules. Low severity because the gap is workspace-wide; this task tracks the per-crate fix.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either [lints] workspace = true is added once [workspace.lints] is established, or this task is closed as duplicate of the workspace-wide rollout task
- [ ] #2 cargo clippy -p ops-test-coverage honours the workspace lint set
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Deferred: root Cargo.toml has no [workspace.lints] section. Per-crate [lints] workspace = true cannot be applied until the workspace-wide lint rollout establishes the shared config. No sibling extensions-rust crate inherits lints either — this is a workspace-wide gap, not a per-crate miss.
<!-- SECTION:NOTES:END -->
