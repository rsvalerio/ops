---
id: TASK-2606
title: 'FN-1: six production functions in ops-about exceed the 50-line threshold'
status: Done
assignee: []
created_date: '2026-10-10 20:50'
updated_date: '2026-10-10 21:53'
labels:
  - code-review-rust
  - fn
dependencies: []
parent_task_id: 'TASK-2623'
modified_files:
  - extensions/about/src/text_util.rs
  - extensions/about/src/units.rs
  - extensions/about/src/machine.rs
  - extensions/about/src/manifest_cache.rs
  - extensions/about/src/cards.rs
priority: medium
ordinal: 1000
dedup_key: 'FN-1:extensions/about/src/text_util.rs:wrap_text'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/text_util.rs:235` (worst instance; full list below)

<!-- scan confidence: candidates to inspect — line spans include doc comments; judge code extent -->

**What**: Production functions over the ~50-line threshold, listed longest first:

- `src/text_util.rs:235` `wrap_text` (~91 lines) — mixes word-accumulation, the max-lines cap with dropped-tail tracking, the ellipsis fixup, and the final per-line width enforcement in one body; the two fixup passes after the loop are extractable helpers.
- `src/units.rs:245` `enrich_from_db` (~88 lines) — four near-identical `match query { Ok/Err(push partial_failures) }` blocks followed by the application loop; a small `try_query(label, query)` helper would collapse them.
- `src/machine.rs:594` `resolve_cargo_settings` (~85 lines) — sequential resolution of ten fields in one body (env fallback chains inline per field); each field's resolution is a named helper away from the rest.
- `src/manifest_cache.rs:139` `ArcTextCache::read` (~76 lines, roughly half comments) — hit/miss branches around the locked map manipulation and the out-of-lock `get_or_init` read.
- `src/cards.rs:86` `render_card` (~75 lines) — card assembly mixing border construction, title/path truncation, stats line, and description rows.
- `src/machine.rs:968` `collect_machine_report` (~67 lines) — probe sequence plus report assembly (linear, single level; weakest candidate).

**Why it matters**: FN-1: functions ≤50 lines, one abstraction level per function. `wrap_text` and `enrich_from_db` in particular carry multiple responsibilities in one body. The others are linear orchestration and may be accepted at triage; they are listed so the outlier set is visible in one place.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 wrap_text extracts the ellipsis fixup and the per-line width enforcement into named helpers
- [x] #2 enrich_from_db collapses the four query match blocks behind one helper
- [x] #3 Remaining candidates either drop under ~50 lines or get a triaged accept with rationale

<!-- AC:END -->
