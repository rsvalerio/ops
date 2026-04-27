---
id: TASK-0358
title: 'SEC-15: format_duration uses unchecked f64 -> u64 cast'
status: Done
assignee:
  - TASK-0415
created_date: '2026-04-26 09:36'
updated_date: '2026-04-26 11:13'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/step_line_theme.rs:18`

**What**: format_duration casts secs / 60.0 and secs / 3600.0 with as u64. Negative, NaN, or huge f64 inputs silently saturate (since 1.45) producing nonsense like "0m0s" for negative or NaN durations.

**Why it matters**: Step elapsed time comes from system clocks; clock skew or a test passing f64::NAN yields misleading display rather than a panic or marker glyph.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Guard against secs.is_nan() || secs < 0.0 with an explicit display (e.g. -- or clamp to 0.0); use try_from after rounding for the integer parts
- [ ] #2 Add tests for NaN, negative, and f64::INFINITY inputs asserting the chosen fallback
<!-- AC:END -->
