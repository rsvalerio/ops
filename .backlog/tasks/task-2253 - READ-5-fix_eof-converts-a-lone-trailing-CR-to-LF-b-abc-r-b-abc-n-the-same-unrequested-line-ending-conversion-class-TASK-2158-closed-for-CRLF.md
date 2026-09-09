---
id: TASK-2253
title: >-
  READ-5: fix_eof converts a lone trailing CR to LF (b"abc\r" -> b"abc\n"), the
  same unrequested line-ending conversion class TASK-2158 closed for CRLF
status: Triage
assignee: []
created_date: '2026-09-08 16:14'
labels:
  - code-review-rust
  - correctness
dependencies: []
modified_files:
  - extensions/text-fixers/src/eof.rs
priority: low
ordinal: 159000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/text-fixers/src/eof.rs:22-35`

**What**: the trailing-terminator walk in `fix_eof` strips both `\n` and `\r`, and `detect_crlf` (post-TASK-2158, correctly measuring the whole input) still counts zero CRLF/LF pairs for an input like `b"abc\r"`, so LF is appended and the lone CR is dropped: `fix_eof(b"abc\r") == Some(b"abc\n")`. That is a byte-level line-ending conversion of a lone-CR (classic Mac) terminator, performed unasked on the pre-commit path — the same class of conversion TASK-2158 just closed for single-line CRLF files.

**Why it matters**: TASK-2158's module-header rule says treating `\r` as convertible payload is out of scope for a whitespace fixer. The lone-CR case escaped because its ACs covered only CRLF pairs and the no-terminator case. A decision is needed: either a lone trailing CR is a terminator to preserve (`fix_eof(b"abc\r")` -> `None` or `Some(b"abc\r")`), or the conversion is deliberate and the module header must say so. Lone-CR files are rare enough that triage may legitimately downscore this, but the behaviour is currently undocumented either way.

**Origin**: discovered during TASK-2237 while fixing TASK-2158.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A recorded decision and test pin what fix_eof does with a lone trailing CR, and the module header documents whichever behaviour was chosen
<!-- AC:END -->
