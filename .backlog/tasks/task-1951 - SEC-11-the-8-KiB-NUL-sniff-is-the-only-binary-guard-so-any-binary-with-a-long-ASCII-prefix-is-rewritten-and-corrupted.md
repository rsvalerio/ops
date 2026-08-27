---
id: TASK-1951
title: >-
  SEC-11: the 8 KiB NUL sniff is the only binary guard, so any binary with a
  long ASCII prefix is rewritten and corrupted
status: Triage
assignee: []
created_date: '2026-08-27 15:49'
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
- [ ] #1 The binary guard used by run_fixer inspects the entire buffer, not a fixed 8 KiB prefix
- [ ] #2 A non-UTF-8 buffer is either rejected as non-text or the docs state explicitly why it is fixed anyway
- [ ] #3 An end-to-end test writes a fixture that is ASCII for more than 8 KiB followed by binary payload bytes containing a 0x20 0x0A pair, runs both fixers, and asserts the file is byte-identical afterwards
- [ ] #4 binary.rs test nul_outside_sniff_window_is_not_binary is updated or removed so it no longer asserts the corrupting behaviour
<!-- AC:END -->
