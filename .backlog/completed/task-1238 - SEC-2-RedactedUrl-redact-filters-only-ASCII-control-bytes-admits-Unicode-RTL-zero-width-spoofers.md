---
id: TASK-1238
title: >-
  SEC-2: RedactedUrl::redact filters only ASCII control bytes, admits Unicode
  RTL/zero-width spoofers
status: Done
assignee:
  - TASK-1259
created_date: '2026-05-08 12:59'
updated_date: '2026-05-08 13:41'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:64-66`

**What**: `is_ascii_control_byte` rejects bytes <0x20 and 0x7f only. Multibyte sequences for RIGHT-TO-LEFT OVERRIDE (U+202E), zero-width joiners (U+200B/200D), BOM (U+FEFF), and other Unicode formatting characters survive redaction and reach about cards / JSON / logs through `RedactedUrl::as_str`.

**Why it matters**: Bidi/homograph spoofing of remote host/owner in operator-facing surfaces, bypassing the SEC-2 control-byte hardening from TASK-1102. TASK-1165 covers silent concatenation but not the bidi/zero-width admission angle.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Reject Unicode formatting/separator categories before constructing RedactedUrl
- [x] #2 Tests pinning rejection of U+202E, U+200B, U+200D, U+FEFF inside url= values
- [x] #3 Document the broader codepoint policy in RedactedUrl::redact
<!-- AC:END -->
