---
id: TASK-0266
title: >-
  TEST-1: hide_irrelevant_commands_preserves_non_stack_commands filters
  hand-picked names only
status: Done
assignee: []
created_date: '2026-04-23 06:36'
updated_date: '2026-04-23 15:22'
labels:
  - rust-code-review
  - test
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/args.rs:358`

**What**: Test only asserts on init/about/theme/extension; a new non-stack built-in added without test update escapes.

**Why it matters**: Regression masked by the subset filter.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Iterate all non-stack built-ins
- [ ] #2 Fail if any non-stack cmd is hidden
<!-- AC:END -->
