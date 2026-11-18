---
id: TASK-2704
title: 'READ-10: stale #[allow(dead_code)] on WorkspaceCanonicalCache::clear and impl ExecTaskCtx, both now used'
status: Done
assignee: []
created_date: '2026-11-18 18:31'
updated_date: '2026-11-18 21:22'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2842'
modified_files:
  - crates/runner/src/command/build.rs
  - crates/runner/src/command/exec.rs
priority: low
ordinal: 1000
dedup_key: 'READ-10:crates/runner/src/command/build.rs:WorkspaceCanonicalCache::clear'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/build.rs:97` (`WorkspaceCanonicalCache::clear`), `crates/runner/src/command/exec.rs:765` (`impl ExecTaskCtx` / `ExecTaskCtx::new`)

**What**: Both items carry `#[allow(dead_code)]`, but both are now used in production: `clear` by the public `CommandRunner::clear_workspace_cache` (`mod.rs:323`), and `ExecTaskCtx::new` by `matrix::run_matrix` (`matrix.rs:168`). The suppressions no longer hide anything.

**Why it matters**: An `#[allow]` that has outlived its reason hides the next time the item really does go dead. READ-10: use `#[expect(dead_code, reason = …)]` for suppressions tied to a specific violation, so `unfulfilled_lint_expectations` would have flagged both of these once they stopped firing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Both #[allow(dead_code)] attributes are removed (or replaced by #[expect(..., reason)] only where the lint still fires under some cfg), and the crate builds warning-free under the workspace lint policy

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.05 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
