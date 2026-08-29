---
id: TASK-2057
title: >-
  DUP-1: the terraform heredoc opener is recognised twice, in strip_comments and
  scan_line
status: Triage
assignee: []
created_date: '2026-08-29 13:42'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions-terraform/about/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs` (`strip_comments`, `scan_line`)

**What**: TASK-2031 made the HCL scanner heredoc-aware, but the two stages
recognise the opener independently. `scan_line` parses `<<[-]?IDENT` from a
`&str` via `heredoc_terminator`; `strip_comments`, which walks a
`Peekable<Chars>` and has no slice to hand it, re-implements the same walk
inline. Only the two character-class predicates (`is_heredoc_ident_start` /
`is_heredoc_ident_char`) are shared.

**Why it matters**: the two recognisers must agree or the pipeline splits:
`strip_comments` passing a body through verbatim while `scan_line` does not
consider itself inside a heredoc puts raw shell text back into the structural
scan, which is exactly the failure TASK-2031 fixed. Nothing pins them
together — no test drives one recogniser against the other — so a later
extension (`<<"EOT"`, a wider terminator charset) can be applied to one side
alone and pass the suite.

**Fix direction**: give `strip_comments` an index-based scan over the content
so it can call `heredoc_terminator` on the remaining slice, or lift the
heredoc state machine into one type both stages drive.

**Origin**: discovered during TASK-2050 while fixing TASK-2031.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The heredoc opener grammar is stated once and used by both strip_comments and scan_line
- [ ] #2 A test pins the two stages to the same opener set
<!-- AC:END -->
