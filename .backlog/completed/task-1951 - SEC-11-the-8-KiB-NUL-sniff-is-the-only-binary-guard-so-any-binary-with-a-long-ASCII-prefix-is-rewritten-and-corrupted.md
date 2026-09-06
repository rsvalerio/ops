---
id: TASK-1951
title: >-
  SEC-11: the 8 KiB NUL sniff is the only binary guard, so any binary with a
  long ASCII prefix is rewritten and corrupted
status: Done
assignee:
  - TASK-2011
created_date: '2026-08-27 15:49'
updated_date: '2026-08-28 23:36'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/text-fixers/src/binary.rs
  - extensions/text-fixers/src/lib.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Severity**: High

**File**: `extensions/text-fixers/src/binary.rs:4-12` (`is_probably_binary`), consumed at `extensions/text-fixers/src/lib.rs:135`

**What**: the sole format check before rewriting a file is "is there a 0x00 byte in the first 8 KiB". Anything else is treated as text and handed to `fix_trailing` / `fix_eof`, which then edit it byte-wise and write it back. The crate's own test `nul_outside_sniff_window_is_not_binary` (binary.rs:38-43) documents the hole and asserts the wrong-for-this-caller behaviour is intentional.

Concrete corruption cases, all of which are plain files that commonly live in a repo:

- **Netpbm images** (`P5`/`P6` .pgm/.ppm): ASCII header, then a raw pixel payload. Any payload byte 0x20 or 0x09 that happens to precede a 0x0A is stripped by `fix_trailing`; `fix_eof` will also strip trailing 0x0A/0x0D pixel bytes and append one. Test fixtures and golden images are exactly this.
- **PDF, PostScript, and uncompressed `.ps`/`.eps`**: long ASCII preamble, binary streams later.
- **Any archive or blob whose first 8 KiB is a text manifest** — some `.tar` files (512-byte ASCII headers, and a NUL-free short text member can push the first binary bytes past 8 KiB), font and asset containers with textual headers.
- **A NUL-free binary format outright** (e.g. some base-N or high-bit encodings) is never detected at all, at any offset.

Note the two fixers are not symmetric in blast radius: `fix_eof` only touches the tail, but `fix_trailing` walks the whole file and deletes bytes anywhere, so the corruption is spread through the payload and is not recoverable by inspection.

**Why it matters**: this is byte-level corruption of the user's files on an automatic pre-commit path, and unlike the truncation risk in TASK-1943 it produces a file that still *opens* — the damage is discovered much later. The fix is cheap and the current heuristic is weaker than the `git`/`pre-commit-hooks` behaviour the module doc claims to match: git's `buffer_is_binary` is used against the *whole* blob it has in memory, not a fixed prefix of a file it is about to rewrite.

**Suggested fix**: since `run_fixer` already reads the entire file into memory before calling this (lib.rs:129), scan the whole buffer for NUL rather than the first 8 KiB — the sniff window buys nothing here. Additionally reject buffers that are not valid UTF-8 (both fixers only reason about ASCII whitespace, so restricting to UTF-8 is a strict safety win and matches "text fixer"), or explain in the docs why non-UTF-8 legacy encodings must stay in scope. If the sniff window is kept for a streaming caller, make that caller explicit and give `run_fixer` the full-buffer variant.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The binary guard used by run_fixer inspects the entire buffer, not a fixed 8 KiB prefix
- [x] #2 A non-UTF-8 buffer is either rejected as non-text or the docs state explicitly why it is fixed anyway
- [x] #3 An end-to-end test writes a fixture that is ASCII for more than 8 KiB followed by binary payload bytes containing a 0x20 0x0A pair, runs both fixers, and asserts the file is byte-identical afterwards
- [x] #4 binary.rs test nul_outside_sniff_window_is_not_binary is updated or removed so it no longer asserts the corrupting behaviour
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in TASK-2011. `binary::is_probably_binary` is replaced by `binary::is_text`, which scans the **entire** buffer for NUL and additionally requires valid UTF-8. `run_fixer` reads the whole file before asking, so the 8 KiB sniff window bought nothing and cost correctness.

AC#2: non-UTF-8 is rejected as non-text. The module doc records the trade — both fixers reason only about ASCII whitespace, so restricting to UTF-8 rules out NUL-free binary formats that the NUL test cannot detect at any offset, at the cost of leaving a legacy single-byte-encoded text file untouched. Not fixing a file is recoverable; corrupting one is not.

AC#4: `nul_outside_sniff_window_is_not_binary` is replaced by `nul_far_past_the_old_8_kib_sniff_window_is_still_not_text`, which asserts the opposite, plus `nul_free_non_utf8_is_not_text`.

AC#3: `tests::a_binary_payload_past_the_old_sniff_window_is_left_byte_identical` — 9000 ASCII bytes, a `P5` header, then `0x20 0x0A 0xFF 0x00 0x20 0x0A`; both fixers run and the file is asserted byte-identical.

Non-text files are counted in `FixerReport::files_skipped` and shown in the summary, but not listed one line per file: every file in the tree is a candidate here (unlike the config checkers, which filter by extension), so per-file lines for every image and font would bury the skips that matter. Rationale is in `runner::write_skip`.
<!-- SECTION:NOTES:END -->
