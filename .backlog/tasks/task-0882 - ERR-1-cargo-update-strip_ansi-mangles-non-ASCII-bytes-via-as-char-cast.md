---
id: TASK-0882
title: 'ERR-1: cargo-update strip_ansi mangles non-ASCII bytes via ''as char'' cast'
status: Done
assignee: []
created_date: '2026-05-02 09:36'
updated_date: '2026-05-02 14:40'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:156-178`

**What**: `strip_ansi` iterates `s.as_bytes()` and emits each non-ESC byte via `result.push(bytes[i] as char)`. Casting a `u8` to `char` interprets the byte as a Unicode code point (U+0000..U+00FF), so any UTF-8 multi-byte sequence (e.g. crate names with non-ASCII characters in cargo-update output, or any operator/locale-emitted UTF-8 in stderr) is silently corrupted: each continuation byte becomes a Latin-1 code point and the original character is lost. The function is documented as "Strip ANSI escape sequences from a string" but actually performs a lossy ASCII-projection of the entire input.

**Why it matters**: Although crate names are ASCII per cargo policy, the stderr stream this parser consumes is not guaranteed to be — a localized rustc/cargo, a crate with non-ASCII metadata in the line, or any non-ASCII in tracing diagnostic lines causes silent character corruption that flows into `parse_action_line` and `tracing::warn!` lines emitted from `parse_update_output`. Equivalent ANSI-strippers in this codebase (e.g. `ops_core::style::strip_ansi`) iterate `chars()` precisely to avoid this. Replace the inner branch with `result.push_str` of the matching `char`, or iterate `s.chars()` and re-encode the ESC[ CSI state machine on a `char`-stream.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 strip_ansi preserves non-ASCII UTF-8 input identically (round-trips through the function)
- [x] #2 an existing project ANSI stripper is reused or this implementation iterates chars() rather than bytes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Rewrote strip_ansi to iterate `chars()` instead of bytes — non-ASCII UTF-8 input now round-trips. Did not pull in ops_core::style as a dep just for this single use; the inline state machine stays narrow. Added 3 regression tests pinning round-trip on cafe-style strings, CSI removal around Unicode, and CSI termination safety with a non-ASCII follow-on byte.
<!-- SECTION:NOTES:END -->
