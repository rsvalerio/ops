---
id: TASK-1498
title: >-
  ARCH-1: cargo-toml lib.rs is 610 lines mixing extension wiring, provider, and
  workspace-root discovery
status: To Do
assignee:
  - TASK-1573
created_date: '2026-05-18 18:03'
updated_date: '2026-05-19 16:45'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/lib.rs:1-610`

**What**: `lib.rs` hosts (a) crate-level docs, (b) `FindWorkspaceRootError` definition, (c) `CargoTomlExtension` + factory, (d) `CargoTomlProvider` + `DataProvider` impl + schema, (e) `find_workspace_root` + strict variant + `manifest_declares_workspace`. Five unrelated concerns past the ARCH-1 red-flag threshold (>500 lines).

**Why it matters**: ARCH-8 prescribes `lib.rs` as a thin entry point; the discovery logic in particular (~280 lines including doc comments) is independently testable and would simplify navigation for future SEC-25 work. `inheritance.rs` and `types.rs` already follow this split — `lib.rs` is the outlier.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Workspace-root discovery (both find_workspace_root* fns + manifest_declares_workspace + MAX_ANCESTOR_DEPTH + FindWorkspaceRootError) extracted to src/workspace_root.rs (or similar concern-named module per ARCH-4)
- [ ] #2 lib.rs retains module declarations, re-exports, extension wiring, and the DataProvider impl only
- [ ] #3 Public API surface preserved via pub use re-exports; all tests still pass
<!-- AC:END -->
