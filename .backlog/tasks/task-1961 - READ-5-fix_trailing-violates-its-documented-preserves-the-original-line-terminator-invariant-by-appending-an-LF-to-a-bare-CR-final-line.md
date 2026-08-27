---
id: TASK-1961
title: >-
  READ-5: fix_trailing violates its documented 'preserves the original line
  terminator' invariant by appending an LF to a bare-CR final line
status: Triage
assignee: []
created_date: '2026-08-27 15:51'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions/text-fixers/src/trailing.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Severity**: Medium

**File**: `extensions/text-fixers/src/trailing.rs:1-3` (module doc) and `:23-58` (`fix_trailing`)

**What**: the module doc states "Preserves the original line terminator (LF or CRLF)". The code does not. When the input's final line ends in a bare CR with no LF, the CRLF branch fires anyway and writes a two-byte `\r\n` where the input had one byte.

`has_crlf` (line 23) is computed as `line_end > start && input.get(line_end - 1) == Some(&b'\r')`. In the no-newline-found branch, `line_end` is set to `input.len()` (line 16), so the last byte of the *file* is tested as though it were the byte before a newline. A trailing `\r` therefore sets `has_crlf`, and line 55 emits `b"\r\n"`.

Verified by compiling `fix_trailing` standalone and running it:

    "abc \r"   -> Some("abc\r\n")      // space stripped AND an LF invented
    "a \rb \r" -> Some("a \rb\r\n")    // CR-only file: interior trailing space NOT stripped, LF appended
    "abc\r"    -> None                 // same input shape, no change at all
    "x  \n"    -> Some("x\n")          // control

The `"abc\r"` case is the tell: whether the spurious LF is written depends on whether some *other* part of the line happened to need trimming, so the bug is invisible until a file has both properties. CR-only line endings are the classic case (classic-Mac exports, some hardware and instrument output, `.strings`-style resources, hand-written fixtures testing line-ending handling — note this crate's own `eof.rs` has a `detect_crlf` that would classify such a file as LF-dominant), but any file whose last byte is `\r` hits it.

For a CR-only file there is a second half to the same defect: because no `\n` is ever found, the whole file is treated as one line, so trailing whitespace *before* each interior CR is never stripped — the fixer silently under-performs while also modifying the byte count.

**Why it matters**: a file-rewriting tool that changes bytes its documentation promises not to touch. It is small in isolation, but it is the kind of thing that shows up as a mysterious one-byte diff in a binary-ish fixture, and READ-5 exists precisely for invariants that the docs assert and the code does not enforce.

**Suggested fix**: only treat a `\r` as part of a terminator when a `\n` was actually found — i.e. gate `has_crlf` on `nl.is_some()`. Then decide, and document, what a bare-CR line terminator means for this fixer: either treat CR as a line terminator throughout (splitting on it, so interior trailing whitespace is stripped) or state that CR-only files are out of scope and leave them entirely untouched. Whichever way, the module doc and the code must agree.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 fix_trailing no longer appends a newline byte that was not present in the input, verified by a test asserting fix_trailing on 'abc \r' does not grow the file
- [ ] #2 A test covers the CR-only line-ending case and asserts the documented behaviour, whichever behaviour is chosen
- [ ] #3 The trailing.rs module doc and the code agree on how a bare CR is treated
- [ ] #4 A property or round-trip test asserts that for any input, the output of fix_trailing contains exactly the same number of 0x0A bytes as the input
<!-- AC:END -->
