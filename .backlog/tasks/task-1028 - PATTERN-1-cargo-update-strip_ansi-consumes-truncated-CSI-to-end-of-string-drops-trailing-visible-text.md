---
id: TASK-1028
title: >-
  PATTERN-1: cargo-update strip_ansi consumes truncated CSI to end of string,
  drops trailing visible text
status: Done
assignee: []
created_date: '2026-05-07 20:23'
updated_date: '2026-05-07 23:29'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:174-191`

**What**: `strip_ansi` enters CSI consumption when it sees `ESC [`, then loops `for next in chars.by_ref()` until a final byte in `0x40..=0x7E` is encountered. If the input ends mid-CSI (a truncated stderr line — pipe close, log rotation, network drop), the inner `for` loop drains the rest of `chars` without ever finding a final byte, silently swallowing every visible character that happened to follow the orphaned `\x1b[` *up to end-of-string*. There is no breadcrumb for the malformed-CSI case, so a truncation bug upstream becomes a missing-data bug here.

The dual ERR-1 / TASK-0681 / TASK-0882 history acknowledges the previous narrow-byte / SGR-only versions were wrong — this one handles all CSI shapes correctly when input is well-formed, but is silently lossy on truncated input.

**Why it matters**: `parse_update_output` operates on cargo's stderr which can be truncated in CI when subprocess output is piped through `format_error_tail` or sliced by parents. A truncated `Updating ... \x1b[3` followed by EOL would cause the final `m` to never appear; the rest of stderr (further `Updating` lines, error message tails) would be consumed silently inside the CSI loop and the resulting empty `clean` line would be skipped, undercounting updates.

**Suggested fix**: on `chars` exhausting inside the CSI loop, append the buffered prefix back into `result` (or warn at debug) so truncated input degrades to "show the raw bytes" rather than "drop everything to EOF".
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Truncated CSI sequences either preserve trailing characters or emit a tracing::debug breadcrumb
- [ ] #2 Unit test pins behaviour for input ending mid-CSI (e.g. "hi\x1b[3")
<!-- AC:END -->
