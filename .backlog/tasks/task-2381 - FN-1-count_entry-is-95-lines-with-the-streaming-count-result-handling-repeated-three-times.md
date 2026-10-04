---
id: TASK-2381
title: 'FN-1: count_entry is ~95 lines with the streaming-count result handling repeated three times'
status: Triage
assignee: []
created_date: '2026-10-04 14:13'
labels:
  - code-review-rust
  - fn
dependencies: []
modified_files:
  - extensions-rust/loc/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'FN-1:extensions-rust/loc/src/lib.rs:count_entry'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/loc/src/lib.rs:259-355` (`count_entry`)

**What**: `count_entry` runs ~97 lines and reaches nesting depth 4-5 (`match` inside `match` arm inside `else` branch). It mixes entry filtering, path relativizing, file open, size decision, capped read, and streaming fallback. The `count_streaming` result handling (`Ok(Some)` / `Ok(None) => TimedOut` / `Err => warn "skipping unreadable file" + Skipped`) is copied verbatim at lines 306-313 and 338-345, and the "skipping unreadable file" warn appears a third time at 347-349.

**Why it matters**: FN-1/FN-2/DUP-1. The three-way fallback outcome mapping must stay in sync by hand; a change to the warn text or the timeout mapping can silently diverge between the over-cap and grew-after-open paths.

**Suggested shape**: extract `fn streaming_outcome(reader, region, deadline, path) -> Result<FileCounts, EntryCount>` (or return `EntryCount` directly) used by both fallbacks, and split the open+size step into its own helper.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 count_entry is at most 50 lines and nests at most 3 levels
- [ ] #2 The count_streaming Ok(Some)/Ok(None)/Err mapping exists in exactly one helper
- [ ] #3 Existing tests in extensions-rust/loc/src/tests.rs pass unchanged
<!-- AC:END -->
