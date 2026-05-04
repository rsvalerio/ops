---
id: TASK-1001
title: >-
  PERF-3: pad_to_width_plain re-implements width sum per char instead of
  delegating to display_width
status: Triage
assignee: []
created_date: '2026-05-04 22:02'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/text_util.rs:33-40`

**What**: `pad_to_width_plain` computes the input width as `s.chars().map(char_display_width).sum::<usize>()`. The same module already imports `ops_core::output::display_width` (line 5) and uses it in `pad_header`, `wrap_text`, and tests. `char_display_width` is `unicode_width::UnicodeWidthChar::width` — char-by-char — which is *not* equivalent to `display_width(s)` for strings containing emoji ZWJ sequences (`👨‍👩‍👧`), regional-indicator flag pairs, or variation selectors: char-summed widths over-count by the width of every joiner / VS-16 glyph, while `unicode_width::UnicodeWidthStr::width` (used by `display_width` per ops_core convention) handles the cluster correctly.

**Why it matters**:
- About cards render TTY-width-padded titles (`render_card` in `cards.rs:117`) and pad rows of card stats. A unit named with an emoji (`✨ release` / `🚀 service-x`) gets a width-1 codepoint counted as width-2, miscomputing the pad and producing a card that visibly leaks past `inner_width` or sits mis-aligned with siblings on the same row.
- The fix is structurally trivial — replace the per-char sum with `display_width(s)` — and unifies the module on a single width primitive (PATTERN-1 / DUP-1 in spirit). The current shape is also a maintainability hazard: a future change to `display_width` (e.g., to handle a new Unicode 16 grapheme cluster) would silently skip `pad_to_width_plain`.

**Note**: this is a quality-of-rendering issue rather than a correctness bug for ASCII content. Severity is medium because the broken sites are operator-facing TTY UI and the silent drift between two width helpers in the same file is exactly the maintainability trap ARCH-2 / PATTERN-1 calls out.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 pad_to_width_plain delegates to display_width(s) instead of summing char_display_width.
- [ ] #2 Test pins that a string containing an emoji ZWJ sequence (e.g., 👨‍👩‍👧) is padded to the same total width as display_width reports.
<!-- AC:END -->
