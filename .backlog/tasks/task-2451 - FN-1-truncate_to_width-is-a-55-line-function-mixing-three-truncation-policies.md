---
id: TASK-2451
title: 'FN-1: truncate_to_width is a 55-line function mixing three truncation policies'
status: Done
assignee: []
created_date: '2026-10-10 15:24'
updated_date: '2026-10-10 21:25'
labels:
  - code-review-rust
  - structure
dependencies: []
parent_task_id: 'TASK-2609'
modified_files:
  - crates/theme/src/style/strip.rs
priority: medium
ordinal: 1000
dedup_key: 'FN-1:crates/theme/src/style/strip.rs:truncate_to_width'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/style/strip.rs:355-427`

**What**: `truncate_to_width` measures 55 non-comment code lines against FN-1's 50-line ceiling — the longest function in the crate. It interleaves three distinct policies in one body: the fits/borrowed fast path and body-budget arithmetic, the per-piece emission loop with separate Escape/Raw/Char arms (the Raw arm containing a nested per-character control-filtering loop), and the truncation/ellipsis/reset tail.

**Why it matters**: FN-1 — each function should operate at a single abstraction level. The Raw-run control filter (lines 391-400) and the cut-marking logic (lines 406-421) are self-contained policies that named helpers would make independently testable and keep the main loop at the piece level. Context: the function is heavily lattice-tested via the module's proptest corpus, so an extraction is low-risk.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 truncate_to_width's body is at or under 50 code lines, e.g. by extracting the Raw-run control filter and the ellipsis/reset cut policy into named helpers
- [x] #2 The existing strip/truncate tests and the visible_width proptest corpus pass unchanged (behaviour is byte-identical)

<!-- AC:END -->
