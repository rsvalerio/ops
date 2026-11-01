---
id: TASK-2658
title: 'Cargo workspaces phase 2: nested lock agreement and lints sync checks'
status: Triage
assignee: []
created_date: '2026-11-01 17:39'
labels:
  - config
  - rust
  - feature
dependencies: []
modified_files:
  - crates/core/src/config/workspaces.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/workspaces.rs`

**What**: A nested workspace named in `[cargo] workspaces` runs fmt/clippy/build/deps, but the drift between it and the root is unchecked: its Cargo.lock must agree with the root lockfile for shared crates, and its [lints] tables must copy [workspace.lints]. dbsec checks both with ~490 lines of script (scripts/fuzz-lock-check.sh, scripts/lint-sync-check.sh).

**Why it matters**: without a built-in, every repo with a fuzz workspace re-adds the scripts by hand — the same hand-rolling `[cargo] workspaces` removed for the run commands.

**Origin**: phase 2 named in TASK-2636's proposal; filed when TASK-2636 closed so it does not die with it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A nested workspace lock disagreement with the root for a shared crate is reported
- [ ] #2 A nested [lints] table that differs from [workspace.lints] is reported
<!-- AC:END -->
