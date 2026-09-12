---
id: TASK-2259
title: 'FN-1: scan_line grew to 84 lines when strip_comments was decomposed under it'
status: Triage
assignee: []
created_date: '2026-09-10 18:41'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions-terraform/about/src/lib.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:482` (`scan_line`, ends `:565`)

**What**: TASK-2220 pulled the heredoc body tracker out of `strip_comments` into the shared `HeredocTracker` (`lib.rs:398`) and decomposed the strip side into `strip_comments` (24 lines), `strip_one_line` (46), `strip_code_chars` (44) and `push_heredoc_opener` (28) — all under the FN-1 50-line threshold. Its sibling `scan_line` absorbed the shared-tracker handoff in the same commit and is now 84 lines, over that threshold.

**Why it matters**: FN-1 was the readability rule that motivated decomposing `strip_comments` in the first place. The wave's fix moved one side of the pair under the threshold and the other side further over it, so the crate still has exactly one oversized line-scanner — the finding relocated rather than closed. `scan_line` is the more load-bearing of the two (it drives the construct classification the whole extension reports on), so it is the worse of the two places to carry an 84-line body.

The decomposition already demonstrated in `strip_one_line` / `strip_code_chars` / `push_heredoc_opener` is the obvious template: `scan_line` has the same shape (a heredoc-consumes-line early return, then a character walk, then opener detection).

**Origin**: discovered during TASK-2242 (wave 8) while auditing TASK-2220. It is a side effect of that wave's own fix, but no acceptance criterion on TASK-2220 scoped FN-1 to `scan_line`, so it was filed rather than folded in.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 scan_line is under the FN-1 50-line threshold, or is decomposed into named stages that each are
- [ ] #2 The existing scan_line tests, including both_stages_recognise_the_same_heredoc_openers and the indented-terminator cases, still pass unchanged
<!-- AC:END -->
