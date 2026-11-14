---
id: TASK-2158
title: 'READ-5: fix_eof converts CRLF to LF whenever the body holds no newline, rewriting single-line CRLF files it documents as untouched'
status: Done
assignee: []
created_date: '2026-09-08 07:04'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - readability
  - correctness
dependencies: []
parent_task_id: 'TASK-2237'
modified_files:
  - extensions/text-fixers/src/eof.rs
priority: high
ordinal: 71000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/text-fixers/src/eof.rs:6-49` (`fix_eof`, `detect_crlf`)

**What**: `fix_eof` first walks `end` back over the whole trailing run of `\r`
and `\n`, then asks `detect_crlf(body)` — where `body = input[..end]` — which
terminator to re-append. `body` is the content with *every* trailing
terminator already removed, so for any file whose only newline(s) are the
final ones, `body` contains no `\n` at all, `detect_crlf` returns
`crlf(0) > lf(0) == false`, and an **LF** is appended to a file that was CRLF.

Verified by running the crate's own `fix_eof` (temporary probe test, since
reverted):

```
fix_eof(b"abc\r\n")      -> Some("abc\n")      // already correct; rewritten
fix_eof(b"abc\r\n\r\n")  -> Some("abc\n")      // CRLF -> LF
fix_eof(b"\r\n\r\n")     -> Some("\n")         // CRLF -> LF
```

The first line is the worst of the three: `"abc\r\n"` already ends with exactly
one newline, so the documented contract ("Ensure the file ends with exactly one
newline", "CRLF-dominant files keep CRLF") says the answer is `None`. Instead
the fixer reports a change and rewrites the file.

The existing test `crlf_preserved_when_dominant` only covers *multi-line*
inputs (`"a\r\nb\r\n\r\n"`), where `body` still contains an interior `\r\n`, so
the whole single-line class is uncovered.

**Why it matters**: this is an unrequested line-ending conversion performed by
a tool that rewrites the user's files in place, on a pre-commit path, over
every text file in the repository. `trailing.rs`'s module docs state the
crate's position explicitly — "treating `\r` as a terminator would mean
rewriting line endings, which is a conversion, not a whitespace fix" — and
`eof.rs` breaks exactly that rule. Single-line CRLF files are ordinary
(`.bat`/`.cmd` scripts, one-line Windows-authored config, generated
manifests).

It also breaks the exit-code contract for any repo that keeps CRLF through
`.gitattributes` (`* text eol=crlf`, `*.bat text eol=crlf`): git re-materialises
CRLF on checkout, the fixer converts it back to LF on every run, so
`ops verify` / the pre-commit hook reports a change every single time and can
never reach a fixed point. The crate's own
`both_fixers_over_a_mixed_tree_reach_a_fixed_point` test cannot catch this
because tempdirs have no `.gitattributes`.

<!-- scan confidence: verified — reproduced by executing fix_eof directly -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 detect_crlf answers about the terminator style of the input, not of the terminator-stripped body: a file whose only newlines are the trailing run keeps the terminator it had
- [x] #2 fix_eof(b"abc\r\n") returns None — a single-line CRLF file that already ends in exactly one newline is not a change
- [x] #3 fix_eof(b"abc\r\n\r\n") returns Some(b"abc\r\n") and fix_eof(b"\r\n\r\n") returns Some(b"\r\n")
- [x] #4 The file-with-no-terminator case (b"abc") still gets LF, and the choice for that genuinely-ambiguous case is documented in the module header
- [x] #5 Regression tests cover the single-line CRLF class explicitly, alongside the existing multi-line crlf_preserved_when_dominant
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
detect_crlf now measures the input as received (body + trailing terminator run) instead of the stripped body; module header documents the terminator-vs-body rule and the LF choice for the genuinely ambiguous no-terminator case. New tests: single_line_crlf_file_already_correct_is_not_a_change (None), single_line_crlf_file_with_extra_terminators_keeps_crlf, only_crlf_terminators_keeps_crlf, terminatorless_crlf_history_still_gets_lf. All 71 crate tests green; crlf_preserved_when_dominant and the mixed-tree fixed-point test unchanged and passing.
<!-- SECTION:NOTES:END -->
