---
id: TASK-2803
title: 'CL-3: truncate_to_width cuts strings that already fit, so wrap_text ellipsizes exact-width lines'
status: Done
assignee: []
created_date: '2026-11-18 18:46'
updated_date: '2026-11-18 21:36'
labels:
  - code-review-rust
  - CL
dependencies: []
parent_task_id: 'TASK-2858'
modified_files:
  - extensions/about/src/text_util.rs
priority: medium
ordinal: 1000
dedup_key: 'CL-3:extensions/about/src/text_util.rs:truncate_to_width'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/text_util.rs:146-174` (`truncate_to_width`), `:325-331` (`enforce_line_width_limit`)

**What**: `truncate_to_width` always reserves a column for the ellipsis, so a string whose width equals `max_width` is cut: `truncate_to_width("hello", 5) == "hell…"` (pinned by the test `truncate_to_width_exact_fit`). Its doc says the ellipsis is appended "when anything was cut", and it does not state the hidden precondition that callers must first check `display_width(s) > max_width` (which `cards::fit_line` does). `enforce_line_width_limit` does not honor that precondition: it truncates when `measure_width(line) > max_width - 1`, i.e. also at exactly `max_width`. `wrap_text` packs lines up to `<= max_width`, so any line that lands exactly on the width — e.g. `wrap_text("hello", 5, 1)` → `["hell…"]` — loses its last character and gains a false truncation marker.

**Why it matters**: Unit-card descriptions (inner width 30) whose wrapped line fills the card exactly render a spurious ellipsis, telling the reader text was dropped when it was not, and contradicting `wrap_text`'s "every line <= max_width, ellipsis only when truncated" contract. The precondition is implicit (CL-3) and one internal caller already violates it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 truncate_to_width returns its input unchanged when display_width(s) <= max_width (or the precondition is made explicit and every caller guards with > max_width)
- [x] #2 enforce_line_width_limit only truncates lines wider than max_width
- [x] #3 A test asserts wrap_text("hello", 5, 1) == ["hello"]; truncate_to_width_exact_fit is updated to the corrected contract

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.22 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
