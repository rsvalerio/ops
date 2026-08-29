---
id: TASK-1967
title: >-
  SEC-11: the ANSI grammar only recognises 7-bit ESC forms, so 8-bit C1
  introducers and bare C0 bytes survive strip_ansi and measure as zero columns
status: Done
assignee:
  - TASK-1987
created_date: '2026-08-27 15:53'
updated_date: '2026-08-28 19:01'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - crates/theme/src/style/strip.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/style/strip.rs:29-77` (`AnsiVisibleChars`, `visible_width`, `strip_ansi`)

**What**: the iterator only enters escape handling on `if ch != '\x1b'`. Two whole classes of terminal control bytes never reach a consume arm:

1. **8-bit C1 introducers.** U+0080..U+009F are the single-byte equivalents of the two-byte ESC forms the parser does handle: U+009B is CSI (equivalent to `ESC [`), U+009D is OSC (`ESC ]`), U+0090 is DCS, U+009E PM, U+009F APC, U+0084 IND, U+0085 NEL. When the payload is UTF-8 (which it always is here — the input is `&str`), a terminal in 8-bit mode reads `\u{9b}2J` exactly as it reads `\x1b[2J`. `strip_ansi` returns them untouched, and `visible_width` scores each as `c.width().unwrap_or(0)` = 0 columns while the terminal consumes the following bytes as a command. So the payload of a C1 sequence is *also* counted as visible width, double-breaking the layout: the escape acts, and its argument bytes inflate the measured width.

2. **Bare C0 bytes.** `\r`, `\n`, `\x08`, `\x07`, `\x7f` pass straight through both functions. `visible_width` gives them 0 (correct as a width, misleading as a safety claim); a `\r` inside a step label or a report row returns the cursor to column 0 and overwrites the frame; a `\x08` shifts everything after it left by one column relative to what was measured.

Note the two width helpers already disagree here and the crate knows it — `crates/theme/src/style.rs:96-99` documents that `UnicodeWidthStr` and `UnicodeWidthChar` disagree on control bytes and excludes them from the proptest corpus, so the `visible_width == display_width(&strip_ansi(_))` contract is only pinned for input that has no raw control bytes. That is precisely the input class this finding is about.

**Why it matters**: `strip_ansi` is a `pub` cross-crate API (re-exported from `lib.rs`) whose module doc claims it strips ANSI escapes; callers reasonably treat a stripped string as safe to print and safe to measure. It is neither for C1/C0 input. Every boxed-layout right-pad computation (`wrap_step_line`, `wrap_box_content`, `right_pad_with_border`, `build_horizontal_border`) is downstream of the same measurement.

Related but distinct: TASK-1843 covers the same byte classes in `ops_core::ui::sanitise_line`. This one is the theme crate's own independent ANSI parser, which has no overlap with that helper and is used for width math rather than escaping.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 AnsiVisibleChars treats U+0090, U+009B, U+009D, U+009E and U+009F as introducers equivalent to their two-byte ESC forms and consumes their payloads
- [ ] #2 strip_ansi returns a string containing no C0 control byte other than tab, and no C1 code point, for any input
- [ ] #3 the style.rs proptest corpus is extended with raw C0 and C1 bytes and the visible_width vs display_width-of-stripped contract is pinned on that corpus, replacing the current comment that excludes control bytes as a known wart
<!-- AC:END -->
