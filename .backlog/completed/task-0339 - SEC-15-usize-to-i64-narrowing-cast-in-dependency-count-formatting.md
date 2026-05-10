---
id: TASK-0339
title: 'SEC-15: usize-to-i64 narrowing cast in dependency count formatting'
status: Done
assignee:
  - TASK-0415
created_date: '2026-04-26 09:33'
updated_date: '2026-04-26 11:13'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/project_identity/card.rs:41-46`

**What**: format_number(c as i64) casts usize to i64 without bounds checking. On 64-bit hosts a usize > i64::MAX becomes a negative i64, which then triggers the SEC-15 issue in format_number.

**Why it matters**: While dependency counts realistically do not approach 2^63, the unchecked cast is the antipattern SEC-15 flags.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Use i64::try_from(c).unwrap_or(i64::MAX) or render directly via usize formatting
- [ ] #2 Cover with a test for usize::MAX verifying no panic and a sensible string
<!-- AC:END -->
