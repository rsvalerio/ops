---
id: TASK-0344
title: 'SEC-15: terminal-width arithmetic can wrap u16 in default_columns'
status: Done
assignee:
  - TASK-0415
created_date: '2026-04-26 09:34'
updated_date: '2026-04-26 11:13'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/mod.rs:267-271`

**What**: default_columns computes w.0 * 9 / 10 where w.0 is u16. A reported terminal width above ~7281 columns silently overflows (release) or panics (debug).

**Why it matters**: terminal_size returns whatever the underlying syscall reports; some virtualized terminals can return absurd values that become DoS-ish display artifacts or panics.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Use (w.0 as u32) * 9 / 10 and clamp back to u16::MAX, or use saturating_mul
- [ ] #2 Add a unit test feeding a synthetic large width and asserting no panic / sensible cap
<!-- AC:END -->
