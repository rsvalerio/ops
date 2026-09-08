---
id: TASK-2073
title: 'API-5: bare #[must_use] attributes carry no explanatory message'
status: To Do
assignee:
  - TASK-2247
created_date: '2026-09-07 22:56'
updated_date: '2026-09-08 10:59'
labels:
  - code-review-rust
  - api
dependencies: []
modified_files:
  - extensions/about/src/text_util.rs
  - extensions/about/src/cards.rs
  - extensions/about/src/manifest_cache.rs
  - extensions/about/src/lru.rs
priority: low
ordinal: 4000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/text_util.rs:10,29,41,65,93,103,142,228,336`; `extensions/about/src/cards.rs:48,63`; `extensions/about/src/manifest_cache.rs:212,377`; `extensions/about/src/lru.rs:28,52,80,85`

**What**: <!-- scan confidence: candidates to inspect --> Seventeen `#[must_use]` attributes across the crate use the bare form. The bare attribute yields the generic "unused return value that must be used" warning, which tells the reader nothing about what discarding the value would lose.

Non-test candidates:
- text_util.rs: `get_terminal_width` (10), `parse_terminal_width` (29), `trim_nonempty` (41), `contains_control_chars` (65), `has_allowed_url_scheme` (93), `pad_to_width_plain` (103), `truncate_to_width` (142), `wrap_text` (228), `pad_header` (336)
- cards.rs: `format_unit_name` (48), `build_card_stats_line` (63)
- manifest_cache.rs: `ArcTextCache::new` (212), `for_filename` (377)
- lru.rs: `next_lru_tick` (28), `LruVictimQueue::new` (52), `len` (80), `is_empty` (85)

**Why it matters**: API-5 — always use the message form (`#[must_use = "call the query; the result is not cached"`), written as the action the caller forgot, not a restatement of the warning. Note `Option`/`Result` returns need no attribute at all, so none of these are wrong to annotate — they are just under-specified.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every #[must_use] in the crate uses the message form naming the action the caller forgot
- [ ] #2 No bare #[must_use] remains (verifiable with grep -n '#\[must_use\]' extensions/about/src)
<!-- AC:END -->
