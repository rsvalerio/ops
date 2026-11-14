---
id: TASK-2220
title: 'DUP-1: strip_comments re-implements the heredoc body tracking ScanState already owns, inside a 94-line six-variable loop'
status: Done
assignee: []
created_date: '2026-09-08 07:21'
updated_date: '2026-09-10 16:14'
labels:
  - code-review-rust
  - duplication
dependencies: []
parent_task_id: 'TASK-2242'
modified_files:
  - extensions-terraform/about/src/lib.rs
priority: medium
ordinal: 128000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:595` (`strip_comments`), `:267` (`ScanState`), `:330` (`scan_line`)

**What**: TASK-2057 factored the heredoc *opener* grammar into the shared `heredoc_terminator`, but the heredoc *body* state machine is still written twice:

- `scan_line` / `ScanState`: one `heredoc: Option<Heredoc>` field, closed by `open.closes(line)` on a line already split by `str::lines()`.
- `strip_comments`: three more locals — `pending: Option<Heredoc>`, `heredoc: Option<Heredoc>`, and a hand-accumulated `body_line: String` — plus a promotion rule (`pending.is_some() && out.ends_with('\n')`) that exists only because this stage walks `Chars` instead of lines. It reaches the same `Heredoc::closes` by a completely different route.

`strip_comments` is 94 lines (`:595`–`:689`, FN-1 threshold is 50) juggling six mutable locals (`out`, `chars`, `in_string`, `pending`, `heredoc`, `body_line`); `scan_line` is 67. The crate already carries a bespoke test, `both_stages_recognise_the_same_heredoc_openers`, whose entire purpose is to catch the two stages drifting apart — that test is the evidence the duplication is live, not incidental, and it only covers openers, not body/close behaviour.

**Why it matters**: TASK-2031 exists because these two stages *did* disagree about heredocs, and the fix had to land in both. The remaining split keeps that cost: any future change to heredoc semantics (`<<"EOT"`, indent handling, a terminator charset widening) has to be made twice, in two different iteration styles, or the body passed through verbatim by one stage gets read as structure by the other. Line-oriented handling in `strip_comments` — or having it delegate to the same `ScanState` — would collapse `pending`/`body_line`/the promotion rule entirely.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Heredoc body tracking (open, accumulate, close) exists in exactly one place, shared by strip_comments and scan_line
- [x] #2 strip_comments no longer needs the separate pending/body_line locals or the out.ends_with('\\n') promotion rule
- [x] #3 strip_comments is under the FN-1 50-line threshold, or is decomposed into named stages that each are
- [x] #4 The existing heredoc tests, including both_stages_recognise_the_same_heredoc_openers and the CRLF/indented-terminator cases, still pass unchanged

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented by resumed runner: new shared HeredocTracker (open/consume/close + is_open) used by both scan_line and strip_comments; strip_comments rewritten line-oriented (split_inclusive) and decomposed into strip_one_line / strip_code_chars / push_heredoc_opener stages, each under the FN-1 50-line threshold (strip_comments itself is 24). One mechanical test substitution: the both_stages_recognise_the_same_heredoc_openers oracle reads state.heredoc.is_open() instead of the old Option::is_some() — same assertion, field is now the tracker. Added two regression tests pinning the block-comment-straddling-the-heredoc-handoff bytes and CRLF body pass-through; the straddle case exposed a real line-start-classification bug in the first draft (fixed: a line that begins inside a block comment is never classified as heredoc body). 72 tests green, clippy clean.
<!-- SECTION:NOTES:END -->
