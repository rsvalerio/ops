---
id: TASK-0355
title: 'READ-5: strip_ansi only handles CSI m-terminated SGR sequences'
status: Done
assignee:
  - TASK-0418
created_date: '2026-04-26 09:35'
updated_date: '2026-04-27 10:33'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/style.rs:40`

**What**: strip_ansi consumes only ESC [ ... m sequences. Other escape families (OSC ESC ], hyperlinks ESC ]8;;...ESC \\, DCS, or CSI sequences ending in non-m final bytes) are left in the string. Width-sensitive callers then count escape bytes as visible width.

**Why it matters**: If a label or downstream output contains anything but plain SGR, the right border lands in the wrong column.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either widen strip_ansi to skip all ANSI escape introducers (CSI, OSC, DCS, single-shift), or rename it to strip_sgr and document the limitation
- [ ] #2 Add a unit test passing a string containing an OSC hyperlink and assert the visible portion is preserved
<!-- AC:END -->
