---
id: TASK-0338
title: 'SEC-15: format_number panics on i64::MIN due to unchecked negation'
status: Done
assignee:
  - TASK-0415
created_date: '2026-04-26 09:33'
updated_date: '2026-04-26 11:13'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/text.rs:16-18`

**What**: format_number(n) calls itself with -n for negative inputs; -(i64::MIN) overflows (debug panic, release wrap). Reachable from production via card.rs::std_field_specs which formats c as i64 (where c: usize — a hostile/large value can produce a negative i64).

**Why it matters**: A panic in display code can crash ops about rendering; release-mode wrapping silently produces "-9223…" output.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace negation with n.checked_neg() (or i64::MIN special case) and emit a saturated/marked string instead of panicking
- [ ] #2 Add a unit test asserting format_number(i64::MIN) returns a deterministic non-panicking result
<!-- AC:END -->
