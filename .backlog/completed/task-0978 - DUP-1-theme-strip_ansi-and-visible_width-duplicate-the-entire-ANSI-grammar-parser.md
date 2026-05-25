---
id: TASK-0978
title: >-
  DUP-1: theme strip_ansi and visible_width duplicate the entire ANSI grammar
  parser
status: Done
assignee:
  - TASK-1011
created_date: '2026-05-04 21:58'
updated_date: '2026-05-07 19:01'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/style/strip.rs:29-67` and `crates/theme/src/style/strip.rs:69-106`

**What**: `visible_width` and `strip_ansi` carry two byte-for-byte identical copies of the ANSI/CSI/OSC/two-byte-escape state machine — same `match chars.next()` arms (`'['`, `']' | 'P' | 'X' | '^' | '_'`, `'(' | ')' | '*' | ...`, etc.), same termination conditions (`0x40..=0x7E`, `\x07`, `\x1b\\`), differing only in whether each consumed visible char is pushed to a `String` or summed via `UnicodeWidthChar::width`.

**Why it matters**: every future ANSI-grammar fix has to land twice or the two functions silently disagree — exactly the failure mode the existing `visible_width_matches_display_width_of_stripped` regression test (style.rs:57) was added to catch. The contract test only covers a hand-picked corpus; a missed grammar arm in one copy passes the test but breaks the boxed-layout width math at runtime. PERF-3/TASK-0746 already had to swap call sites between the two, and READ-5/TASK-0355 fixed CSI-final handling that originally lived in only one copy. Factor the parser into a single iterator (`fn ansi_events(&str) -> impl Iterator<Item = AnsiEvent>` returning `Visible(char)` / `Escape`) and let both helpers consume it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Single ANSI grammar parser in theme::style::strip used by both visible_width and strip_ansi
- [x] #2 Existing visible_width_matches_display_width_of_stripped contract test still passes
- [x] #3 No behaviour change: corpus + new fuzz/proptest covering CSI/OSC/two-byte-escape edge cases agree
<!-- AC:END -->
