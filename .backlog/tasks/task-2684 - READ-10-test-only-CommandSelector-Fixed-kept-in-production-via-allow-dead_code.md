---
id: TASK-2684
title: 'READ-10: test-only CommandSelector::Fixed kept in production via allow(dead_code)'
status: Done
assignee: []
created_date: '2026-11-18 18:27'
updated_date: '2026-11-18 22:07'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2843'
modified_files:
  - crates/cli/src/hook_shared.rs
priority: low
ordinal: 1000
dedup_key: 'READ-10:crates/cli/src/hook_shared.rs:CommandSelector'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/hook_shared.rs:69` (`CommandSelector::Fixed`), consumed at `crates/cli/src/hook_shared.rs:127`

**What**: `CommandSelector::Fixed(&[String])` is constructed only from tests (`crates/cli/src/pre_hook_cmd.rs:79`, `:104`, `:126`, all inside its `#[cfg(test)] mod tests`). Production builds keep the variant and its match arm alive behind a bare `#[allow(dead_code)]` with no reason, contrary to the workspace policy that exceptions carry `#[expect(.., reason = "..")]` next to the code (docs/clippy.md).

**Why it matters**: The bare allow hides whether the variant is intended production API or a test seam, and it will never warn if a production caller appears or the tests stop using it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The suppression is either removed (variant used in production), or replaced by #[cfg_attr(not(test), expect(dead_code, reason = "..."))] or a #[cfg(test)]-gated variant

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.02 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
