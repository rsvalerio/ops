---
id: TASK-2019
title: >-
  CL-3: tab survives sanitisation and the theme ANSI grammar but measures as
  zero columns, so it still bends the boxed frame
status: Triage
assignee: []
created_date: '2026-08-28 19:28'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - crates/theme/src/style/strip.rs
  - crates/core/src/ui.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/style/strip.rs`, `crates/core/src/ui.rs`

**What**: TASK-1967 made `strip_ansi` / `visible_width` drop every C0 and C1 control character *except* tab, and `ops_core::ui::sanitise_line` (the SEC-21 defence the theme now routes subprocess stderr through, TASK-1965) likewise passes `\t` verbatim. Tab is therefore the one control character that reaches a rendered line — and both width helpers score it as zero columns (`UnicodeWidthChar::width('\t') == None -> 0`), while a terminal advances the cursor to the next multiple of 8.

Consequence: a tab in a captured stderr line, a report detail, or a step label makes the measured width smaller than the painted width. The boxed-frame right pad (`wrap_step_line`, `wrap_box_content`, `right_pad_with_border`) then under-pads and the closing bar lands short — the same class of corruption TASK-1967 removed for `\r` and friends, on the one byte deliberately left in.

**Why it matters**: tabs are common in compiler and test-runner output (rustc notes, `cargo test` panics, Makefile echoes), so this is the ordinary case rather than an adversarial one. It is also the residual half of TASK-1967's guarantee: the module now documents "no C0 other than tab", which is safe as an *escaping* claim but not as a *measurement* one.

**Possible fix**: expand tab to spaces (to the next 8-column stop, or a single space) inside the theme's width/strip pipeline, so measurement and painting agree; or drop it there and leave `sanitise_line`'s tab handling alone.

**Origin**: discovered during TASK-1987 while fixing TASK-1967 and TASK-1965.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 visible_width and strip_ansi agree with what a terminal paints for a string containing a tab, either by expanding it to spaces or by dropping it
- [ ] #2 a test renders a boxed error-block line whose stderr text contains a tab and asserts the line's visible width equals the frame width and ends with the closing bar
<!-- AC:END -->
