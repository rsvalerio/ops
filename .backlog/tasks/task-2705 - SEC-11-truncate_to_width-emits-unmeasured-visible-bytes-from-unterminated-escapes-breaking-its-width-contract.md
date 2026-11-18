---
id: TASK-2705
title: 'SEC-11: truncate_to_width emits unmeasured visible bytes from unterminated escapes, breaking its width contract'
status: To Do
assignee: []
created_date: '2026-11-18 18:31'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - security
dependencies: []
parent_task_id: 'TASK-2845'
modified_files:
  - crates/theme/src/style/strip.rs
  - crates/theme/src/style.rs
priority: medium
ordinal: 1000
dedup_key: 'review-2667.06-1'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/style/strip.rs:352` (`truncate_to_width`), `crates/theme/src/style/strip.rs:399` (`push_filtered_run`)

**What**: `truncate_to_width` promises its result has visible width `<= max_cols`. For an `AnsiPiece::Raw` run (an escape that never terminated: `ESC [` cut off at end of input, an nF scan that broke on an out-of-range byte, a scan that hit `CSI_SCAN_CAP`), `push_filtered_run` drops the introducer (`ESC` / C1 is a droppable control) but pushes every other character of the run verbatim, and **none of them is charged to the `BodyBudget`**. `visible_width` scores Raw runs as zero columns, so the input "fits" while the output contains visible text the budget never saw. The doc comment on `push_filtered_run` also claims "the introducer included ... preserved", which the code does not do.

Reproduced with a scratch binary against the current crate:

```
truncate_to_width("abc\x1b[12345678", 3)  -> "abc[12345678"   visible_width(out) = 12
truncate_to_width("ab\x1b(\u{e9}xyz", 3)  -> "ab(\u{e9}…\x1b[0m"   visible_width(out) = 5
```

**Why it matters**: `truncate_to_width` is the module-level truncation policy every frame wrapper relies on (`wrap_step_line`, `wrap_box_content`, `right_pad_with_border`, `build_horizontal_border`, `render_slot_gated`). Once the ESC is dropped the leftover bytes are ordinary visible text, so a boxed line can overrun its closing `│` and wrap. The sanitised paths (`render::sanitise`) never hand it an ESC, but step labels, command ids and `plan_header_prefix` reach it unsanitised, and it is a public API (`ops_theme::style::truncate_to_width`) used by other crates. The existing proptest in `src/style.rs` checks `visible_width`/`strip_ansi` agreement only and never asserts the truncation bound.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 truncate_to_width's output satisfies visible_width(out) <= max_cols for every input, including unterminated CSI/nF/OSC runs (either charge the Raw run's surviving characters to the budget, or drop the whole Raw run as strip_ansi does)
- [ ] #2 The push_filtered_run doc comment matches what the code does with the introducer
- [ ] #3 A proptest over the existing escape/control/visible chunk strategy asserts visible_width(&truncate_to_width(&s, n)) <= n for arbitrary n, with regression cases for the two inputs above
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.06 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
