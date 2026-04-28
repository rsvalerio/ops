---
id: TASK-0511
title: >-
  PATTERN-1: warn_raw_drops_parallel only inspects top-level resolve, not nested
  composites
status: To Do
assignee:
  - TASK-0536
created_date: '2026-04-28 06:51'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - correctness
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd.rs:231`

**What**: Only the top-level command's parallel flag is checked; a composite that itself contains a nested `parallel = true` composite reaches `--raw` execution without a warning. The multi-command path warns by walking merge_plan's any_parallel; the single-command path lacks the same depth.

**Why it matters**: Users get silent serial timing for nested-parallel composites under --raw.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Walk the expanded composite tree for any parallel=true
- [ ] #2 Warn once if found in nested composites
<!-- AC:END -->
