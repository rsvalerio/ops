---
id: TASK-0224
title: 'READ-5: should_skip only treats literal "1" as skip — undocumented'
status: Done
assignee: []
created_date: '2026-04-23 06:33'
updated_date: '2026-04-23 15:20'
labels:
  - rust-code-review
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/lib.rs:32`

**What**: `is_ok_and(|v| v == "1")` rejects "true", "yes", "TRUE"; doc says "set to 1" but users commonly try other truthy values.

**Why it matters**: Silent user confusion when SKIP_OPS_RUN_BEFORE_COMMIT=true does not skip.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Document the exact match in the HookConfig field doc
- [ ] #2 Accept common truthy strings (1/true/yes) consistently
<!-- AC:END -->
