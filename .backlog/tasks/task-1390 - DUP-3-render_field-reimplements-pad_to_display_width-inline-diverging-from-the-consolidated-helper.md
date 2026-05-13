---
id: TASK-1390
title: >-
  DUP-3: render_field reimplements pad_to_display_width inline, diverging from
  the consolidated helper
status: Done
assignee:
  - TASK-1452
created_date: '2026-05-13 18:03'
updated_date: '2026-05-13 20:35'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/project_identity/card.rs:220-224` and `crates/core/src/project_identity/format.rs` (format_language_breakdown padding loop)

**What**: `render_field` computes `let pad = (max_key_len + 2).saturating_sub(display_width(key))` and then runs `for _ in 0..pad { padded_key.push(' ') }` inline — a re-implementation of `ops_core::output::pad_to_display_width`, which exists precisely to centralise this. The same manual `display_width` + `push(' ')` loop appears a second time in `project_identity/format.rs`'s breakdown formatter.

**Why it matters**: `pad_to_display_width` was extracted (TASK-1235 per its doc) to be the single producer of width-aware padding, so CJK / wide-emoji alignment can't drift. Two inline reimplementations defeat that — any future fix to the helper (e.g. tab handling, ZWJ handling, a new wide-char range) silently skips these callers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace the inline padding loop at card.rs:220-224 with a call to ops_core::output::pad_to_display_width(key, max_key_len + 2)
- [ ] #2 Replace the equivalent inline loop in project_identity/format.rs's language-breakdown formatter with pad_to_display_width
- [ ] #3 Keep existing render_field_aligns_multi_byte_key_by_display_width and the breakdown tests passing; no new behaviour expected, just the same alignment via the canonical helper
<!-- AC:END -->
