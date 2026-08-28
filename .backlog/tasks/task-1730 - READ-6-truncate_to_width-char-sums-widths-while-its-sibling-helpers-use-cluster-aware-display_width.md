---
id: TASK-1730
title: >-
  READ-6: truncate_to_width char-sums widths while its sibling helpers use
  cluster-aware display_width
status: To Do
assignee:
  - TASK-2003
created_date: '2026-08-27 11:12'
updated_date: '2026-08-28 14:15'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions/about/src/text_util.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/text_util.rs:70-89` (`truncate_to_width`), contrast with `extensions/about/src/text_util.rs:53-67` (`pad_to_width_plain`) and `extensions/about/src/text_util.rs:117-123` (`measure_width` / `wrap_text`)

**What**: The module has two different width models and uses them on the same strings.

`pad_to_width_plain` was deliberately converted to `ops_core::output::display_width` (grapheme-cluster aware) — its comment states why: "Char-summing over-counted joiners / VS-16 glyphs and produced misaligned About cards for unit names containing emoji". `wrap_text` measures through `measure_width`, which is also `display_width`.

`truncate_to_width` was left on the old model: it accumulates `char_display_width(c)` per `char` (line 75-86), which is `unicode_width::UnicodeWidthChar::width`. For a ZWJ sequence such as `\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}` (family emoji) that sums 2+0+2+0+2 = 6 columns where `display_width` reports 2. Regional-indicator flag pairs and VS-16 sequences diverge the same way.

Two consequences, both in the card renderer that calls both helpers on the same values (`cards.rs:96-106` guards with `display_width` and then truncates with `truncate_to_width`):

1. **Over-truncation.** The guard `display_width(&title) > inner_width` says the title fits, but if it does not, `truncate_to_width` measures the same string at up to 3x its real width and cuts far earlier than needed.
2. **Cluster splitting.** The per-`char` loop can break *inside* a ZWJ sequence — pushing `\u{1F468}\u{200D}` and then the ellipsis — which renders as a lone man emoji plus a dangling joiner rather than the family glyph, and whose real rendered width is not the width the loop accounted for. That re-introduces the exact card misalignment TASK-1001 fixed on the padding side.

Note the existing test coverage mirrors the gap: `pad_to_width_uses_display_width_for_zwj_sequence` pins the cluster-aware behaviour for padding, and there is no equivalent for truncation — every `truncate_to_width` test uses ASCII.

**Why it matters**: READ-6 (consistent patterns for similar problems) plus a real rendering defect. Two helpers used together on the same string disagree about what a column is, so the `about` cards' documented "every line <= max_width" contract is enforced against two different definitions of width, and emoji-bearing unit names render broken.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 truncate_to_width measures with the same cluster-aware display_width the rest of the module uses, and never cuts inside a grapheme cluster
- [ ] #2 A test truncates a string containing a ZWJ emoji sequence and asserts the result's display_width is within max_width and that the emoji cluster is either kept whole or dropped whole
- [ ] #3 char_display_width is either removed or its remaining callers are documented as intentionally per-char
<!-- AC:END -->
