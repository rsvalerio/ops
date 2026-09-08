---
id: TASK-2214
title: >-
  PATTERN-1: an unterminated heredoc, block comment, string or block in a .tf
  file silently swallows the rest of the scan with no warning
status: To Do
assignee:
  - TASK-2238
created_date: '2026-09-08 07:21'
updated_date: '2026-09-08 10:55'
labels:
  - code-review-rust
  - pattern
dependencies: []
modified_files:
  - extensions-terraform/about/src/lib.rs
priority: medium
ordinal: 124000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:218` (`extract_required_version`), `:330` (`scan_line`), `:595` (`strip_comments`), `:711` (`blank_block_comment`)

**What**: The HCL scanner reports exactly one structural failure — a `}` with an empty brace stack — via `LineScan::Malformed` and a `tracing::warn!("unbalanced closing brace in .tf content; skipping file")`. Every other way the input can be structurally broken ends the scan silently and is indistinguishable from "this file declares no `required_version`":

- **Unterminated heredoc.** `scan_line` sets `state.heredoc = Some(..)` and thereafter returns `LineScan::Continue` for every remaining line. If the terminator never appears (or `heredoc_terminator` accepted an opener HCL would not), the whole rest of the file is consumed as body. `extract_required_version` falls off the end of the `for` loop and returns `None`. No warn.
- **Unterminated `/* … */`.** `blank_block_comment` blanks through to EOF and returns; the doc comment on `strip_comments` calls this "the same behaviour as terraform's own parser", but terraform *errors* on it while this crate silently renders "no version".
- **Unterminated `"` string.** `strip_comments` carries `in_string` **across** lines, so a single unbalanced quote in code disables comment stripping for the remainder of the file. Note `scan_line`'s `in_string` is line-local, so the two stages disagree about where strings end — the same divergence class `both_stages_recognise_the_same_heredoc_openers` was written to guard for openers.
- **Unclosed `{` at EOF.** `extract_required_version` never inspects `state.stack` after the loop, so a file that opened blocks it never closed is accepted as well-formed.

**Why it matters**: `ops about` renders `stack_detail` from whichever `.tf` file first yields a constraint, and `find_required_version` walks *every* `.tf` in the workspace root. A single malformed file makes the About card silently drop the terraform version with nothing in the logs to explain it, while the one failure mode that *is* reported gets a warn. Operators cannot tell "no constraint declared" from "the parser gave up". This is the Terraform twin of TASK-2181 (`extensions-go/about`: an unterminated `go.work` `use (` / `go.mod` `replace (` block silently swallows the rest of the file).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 extract_required_version reports an unterminated heredoc at EOF the same way it reports an unbalanced closing brace (warn + refuse the file), rather than returning None silently
- [ ] #2 An unterminated /* ... */ block comment and an unterminated " string are likewise reported rather than silently blanking or passing through the remainder of the file
- [ ] #3 A non-empty brace stack at EOF is detected and reported instead of being accepted as well-formed
- [ ] #4 strip_comments and scan_line agree on where a string ends (both line-local, or both cross-line) and a test pins the agreed behaviour
- [ ] #5 Tests cover each of the four unterminated shapes: heredoc, block comment, string, and unclosed block
<!-- AC:END -->
