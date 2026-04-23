---
id: TASK-0215
title: >-
  READ-5: expand_to_leaves silently conflates unknown id, cycle, and depth
  exceeded into Option::None
status: To Do
assignee: []
created_date: '2026-04-23 06:32'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/mod.rs:263`

**What**: Three distinct failure modes return None with no caller-visible discriminant; only depth-exceeded logs a warning.

**Why it matters**: Callers re-interpret None as "unknown command", misleading users when the real failure is a cycle or depth bomb.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Change return type to Result<Vec<CommandId>, ExpandError> with variants Unknown/Cycle/TooDeep
- [ ] #2 Update run/run_raw to surface the correct error
<!-- AC:END -->
