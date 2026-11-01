---
id: TASK-2658
title: 'Cargo workspaces phase 2: nested lock agreement and lints sync checks'
status: Done
assignee: []
created_date: '2026-11-01 17:39'
updated_date: '2026-11-01 19:56'
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
- [x] #1 A nested workspace lock disagreement with the root for a shared crate is reported
- [x] #2 A nested [lints] table that differs from [workspace.lints] is reported

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented as ops check-workspaces (read-only, needs no tool): per extra [cargo] workspaces entry, lock agreement (shared-crate name/version/source triples one-directional, no nested-only dependency edge, no missing required edge for repo-local crates, with dbsec's double-read snapshot guard) and bidirectional [lints] vs [workspace.lints] equality compared as policies. Deliberate differences recorded under [cargo] lint_exemptions as workspace -> group.lint -> reason (HISTORY 0.92.0); an exemption licenses omitting only, never setting or keeping a dropped lint. Validated on dbsec: with the one exemption its own script carries (rust.unsafe_code), the built-in agrees with fuzz-lock-check.sh and lint-sync-check.sh on the same tree — both pass; without it, both report the same single finding.
<!-- SECTION:NOTES:END -->
