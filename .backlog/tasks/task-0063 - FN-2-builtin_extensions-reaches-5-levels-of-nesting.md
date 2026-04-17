---
id: TASK-0063
title: 'FN-2: builtin_extensions reaches 5 levels of nesting'
status: To Do
assignee: []
created_date: '2026-04-17 11:30'
updated_date: '2026-04-17 12:07'
labels:
  - rust-codereview
  - fn
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry.rs:57`

**What**: `builtin_extensions` has nested let-some / for / if / if branches giving it 5 levels of nesting (FN-2 threshold is 4).

**Why it matters**: Deep nesting reduces readability and complicates filtering by stack and config-enabled list.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Early-return or extract filter_by_stack and filter_by_enabled helpers
- [ ] #2 Max nesting in builtin_extensions is 4 or fewer
<!-- AC:END -->
