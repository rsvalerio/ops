---
id: TASK-2220
title: 'DUP-1: strip_comments re-implements the heredoc body tracking ScanState already owns, inside a 94-line six-variable loop'
status: To Do
assignee: []
created_date: '2026-09-08 07:21'
updated_date: '2026-09-08 20:00'
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
- [ ] #1 Heredoc body tracking (open, accumulate, close) exists in exactly one place, shared by strip_comments and scan_line
- [ ] #2 strip_comments no longer needs the separate pending/body_line locals or the out.ends_with('\\n') promotion rule
- [ ] #3 strip_comments is under the FN-1 50-line threshold, or is decomposed into named stages that each are
- [ ] #4 The existing heredoc tests, including both_stages_recognise_the_same_heredoc_openers and the CRLF/indented-terminator cases, still pass unchanged
<!-- AC:END -->
