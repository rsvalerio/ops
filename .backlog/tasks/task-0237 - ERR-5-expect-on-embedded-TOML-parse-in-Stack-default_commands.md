---
id: TASK-0237
title: 'ERR-5: expect on embedded TOML parse in Stack::default_commands'
status: To Do
assignee: []
created_date: '2026-04-23 06:34'
updated_date: '2026-04-23 06:46'
labels:
  - rust-code-review
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/stack.rs:125`

**What**: `toml::from_str(toml).expect(...)` panics at runtime if any of the 8 embedded `.default.<stack>.ops.toml` files becomes invalid.

**Why it matters**: Panics are reachable from production code via init_template even though the input is compile-time embedded.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace with compile-time-enforced const-parse via build-script validation test
- [ ] #2 Or return Result and propagate parse error with context
<!-- AC:END -->
