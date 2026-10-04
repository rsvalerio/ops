---
id: TASK-2378
title: 'ERR-14: check_file drops the TOML line/column from a does-not-parse drift message'
status: Triage
assignee: []
created_date: '2026-10-04 14:13'
labels:
  - code-review-rust
  - ERR
dependencies: []
modified_files:
  - extensions-rust/foundation/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'ERR-14:extensions-rust/foundation/src/lib.rs:check_file'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/foundation/src/lib.rs:366` (`check_file`)

**What**: A repo file that fails to parse is reported as `does not parse: {e.message()}`. `toml::de::Error::message()` is only the reason text; the span and source excerpt that `Display` renders are dropped. These files (`deny.toml`, `clippy.toml`, `.config/nextest.toml`, `mise.toml`) are hand-edited.

**Why it matters**: The user sees "does not parse: invalid string" with no line or column, in a file they edited by hand. ERR-14 asks for the failing location on human-edited input.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The does-not-parse drift message carries the TOML line/column (via Display or e.span()) so the editor can locate the error
- [ ] #2 A test with a malformed template-named file asserts the drift message contains the line number
<!-- AC:END -->
