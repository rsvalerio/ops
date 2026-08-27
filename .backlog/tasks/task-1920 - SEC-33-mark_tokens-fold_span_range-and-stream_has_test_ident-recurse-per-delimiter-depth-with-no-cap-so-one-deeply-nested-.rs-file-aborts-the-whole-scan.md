---
id: TASK-1920
title: >-
  SEC-33: mark_tokens, fold_span_range and stream_has_test_ident recurse per
  delimiter depth with no cap, so one deeply nested .rs file aborts the whole
  scan
status: Triage
assignee: []
created_date: '2026-08-27 15:44'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions-rust/loc/src/counter.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/loc/src/counter.rs:245` (`mark_tokens`), `:392` (`stream_has_test_ident`), `:424` (`fold_span_range`)

**What**: All three token walkers recurse once per level of delimiter nesting, with no depth counter and no bail-out:

- `mark_tokens` (line 266-274): `TokenTree::Group(group) => { ...; mark_tokens(&group.stream(), lines, kinds); ... }`
- `fold_span_range` (line 427-431): same shape, recursing into `group.stream()`
- `stream_has_test_ident` (line 406): `TokenTree::Group(group) if stream_has_test_ident(&group.stream()) => return true`

The input is every `.rs` file found under the working directory, and `proc_macro2`'s own lexer is deliberately iterative for exactly this reason - it even ships a hand-written non-recursive `Drop` for `TokenStream` so that deeply nested input does not blow the stack while being freed. That means a perfectly valid, deeply nested file (generated code, a macro-expansion fixture, a checked-in fuzz corpus, minified/concatenated sources) lexes successfully via `TokenStream::from_str` at line 194 and then overflows the stack inside these walkers.

A stack overflow is not a panic: it aborts the process with SIGSEGV and cannot be caught, so it kills the entire `ops` run rather than degrading.

**Why it matters**: The module contract at counter.rs:165-170 states that `count_source` "Never fails: unparseable input degrades to count_fallback rather than dropping the file, so a single nightly-only syntax file cannot zero out a whole crate's numbers." Unbounded recursion breaks that promise in the worst possible way - instead of one file degrading, the whole data-collection run dies with no diagnostic and no partial result. This is the same class of defect as TASK-1809 (--allow-json5 has no recursion limit) and is filed at the same severity.

`syn::parse_file` at line 211 recurses too, but that is upstream; the three walkers above are this crate's own code and are the ones to bound.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 mark_tokens, fold_span_range and stream_has_test_ident carry an explicit depth parameter (or an equivalent iterative worklist) with a documented maximum nesting depth
- [ ] #2 Exceeding the depth cap makes count_source degrade to count_fallback for that file and emit a tracing::warn naming the limit, instead of recursing further
- [ ] #3 A regression test builds a source string with nesting well past the cap, calls count_source, and asserts it returns non-zero counts rather than aborting the test process
- [ ] #4 The 'Known limits' section of the counter.rs module doc records the depth cap alongside the existing macro-expansion and inline-mod limits
<!-- AC:END -->
