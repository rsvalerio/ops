---
id: TASK-0857
title: 'ERR-5: format_duration uses fragile ''as i128'' saturation idiom'
status: Triage
assignee: []
created_date: '2026-05-02 09:18'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/step_line_theme.rs:27`

**What**: u64::try_from(secs.trunc() as i128) relies on f64 -> i128 saturating cast plus try_from. The intent (saturate huge f64 to u64::MAX) is correct but hidden. A reviewer cannot tell whether the saturation is intended or accidental.

**Why it matters**: READ-5 / CL-3 say preconditions should be explicit. Easy to break in a future refactor.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace with explicit clamp: let clamped = secs.trunc().clamp(0.0, u64::MAX as f64); let total_secs = clamped as u64;
- [ ] #2 Add unit tests for f64::INFINITY-adjacent (1e30) and exact u64::MAX boundaries
- [ ] #3 Drop the i128 indirection
<!-- AC:END -->
