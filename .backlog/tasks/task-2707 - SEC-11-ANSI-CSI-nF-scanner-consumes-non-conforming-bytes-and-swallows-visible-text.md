---
id: TASK-2707
title: 'SEC-11: ANSI CSI/nF scanner consumes non-conforming bytes and swallows visible text'
status: To Do
assignee: []
created_date: '2026-11-18 18:32'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - security
dependencies: []
parent_task_id: 'TASK-2845'
modified_files:
  - crates/theme/src/style/strip.rs
priority: low
ordinal: 1000
dedup_key: 'review-2667.06-2'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/style/strip.rs:199` (`AnsiPieces::consume_csi`), `crates/theme/src/style/strip.rs:240` (`AnsiPieces::consume_nf`)

**What**: `consume_csi` accepts *any* character as a CSI parameter/intermediate byte (`Some(_) => {}`) until it meets a byte in `0x40..=0x7E` or hits `CSI_SCAN_CAP` (64). ECMA-48 restricts parameter bytes to `0x30..=0x3F` and intermediates to `0x20..=0x2F`; anything else means the sequence is malformed. So a stray `ESC [` in captured text followed by non-ASCII or punctuation swallows everything up to the next ASCII letter: `"\x1b[日本語abc"` is parsed as one escape `ESC [ 日本語 a`, `visible_width` reports 2 and `strip_ansi` returns `"bc"`. `consume_nf` has the related problem: the out-of-range byte that proves "this was never an escape" is folded into the Raw run instead of being left for the main loop, so `strip_ansi("ab\x1b(\u{e9}xyz")` drops the `\u{e9}`.

**Why it matters**: The module docs already reject this for `ESC` + space ("consuming it would swallow the following visible word") but the CSI path does the same thing on a wider range of input. This is the workspace's only ANSI grammar: `strip_ansi_preserving_raw` (used by `ops-cargo-update` on untrusted subprocess output) silently loses the swallowed text, and width measurement disagrees with what a terminal prints for the same bytes, misaligning the boxed frame.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 consume_csi ends the sequence as a Raw piece (without consuming it) on the first byte outside 0x20..=0x3F that is not a final byte, so following visible text is re-emitted by the main loop
- [ ] #2 consume_nf leaves the out-of-range byte unconsumed so it is re-emitted as a Char piece
- [ ] #3 Unit tests cover ESC [ followed by non-ASCII text and ESC ( followed by a non-ASCII char for strip_ansi, strip_ansi_preserving_raw and visible_width
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.06 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
