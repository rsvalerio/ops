---
id: TASK-0759
title: >-
  READ-7: run_tools_list_to wildcard match on ToolStatus defeats exhaustive enum
  coverage
status: Done
assignee:
  - TASK-0828
created_date: '2026-05-01 05:54'
updated_date: '2026-05-02 08:00'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/tools_cmd.rs:32-44`

**What**: Two match tool.status blocks list Installed, NotInstalled, Unknown, then add `_ => dim("?")` and `_ => " (UNKNOWN)"`. ToolStatus today has exactly those three variants; the wildcard arm is dead. If a fourth variant is added, both arms silently render it as "Unknown" instead of failing compilation.

**Why it matters**: Compile-time exhaustiveness is the canonical Rust safety net for enum extension. Wildcard hides the obligation to update display logic when ToolStatus grows.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Both match arms enumerate every ToolStatus variant explicitly; the wildcard is removed
- [ ] #2 If #[non_exhaustive] is on ToolStatus, the wildcard is replaced with an explicit fallback that includes the variant via Debug, not silent collapse
- [ ] #3 Adding a new variant to ToolStatus produces a compile error in run_tools_list_to until the new arm is rendered
<!-- AC:END -->
