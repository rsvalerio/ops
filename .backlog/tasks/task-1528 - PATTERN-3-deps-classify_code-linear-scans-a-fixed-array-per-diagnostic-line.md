---
id: TASK-1528
title: 'PATTERN-3: deps classify_code linear-scans a fixed array per diagnostic line'
status: To Do
assignee:
  - TASK-1648
created_date: '2026-05-19 07:33'
updated_date: '2026-05-25 16:08'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse/deny.rs:48-53`

**What**: `classify_code` walks the 14-entry `CODE_CLASSES` slice for every parsed diagnostic. A `match` on the `&str` (or a `phf`/`OnceLock<HashMap>`) gives O(1) dispatch and lets the compiler enforce exhaustiveness when a new code is added.

**Why it matters**: Small N today, but the data-driven slice silently allows duplicate entries and forgoes the compile-time check that's the whole point of FN-1 / TASK-0793's centralisation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace CODE_CLASSES + classify_code with a match expression on the code string
- [ ] #2 Adding a new code is still a one-line edit
- [ ] #3 Existing parse_deny_* tests pass unchanged
<!-- AC:END -->
