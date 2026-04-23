---
id: TASK-0199
title: >-
  TEST-17: tokei_provider_returns_valid_json_on_real_project scans workspace at
  test time
status: Done
assignee: []
created_date: '2026-04-22 21:26'
updated_date: '2026-04-23 08:50'
labels:
  - rust-code-review
  - TEST
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/tests.rs:39-57`

**What**: Test calls TokeiProvider.provide on CARGO_MANIFEST_DIR, running tokei across the whole crate dir each invocation. Couples the test to live filesystem content/order, slows the suite, would fail in sandboxes hiding source files.

**Why it matters**: TEST-17 — no real filesystem scans in unit tests. Test asserts only that the array is non-empty and schema fields exist; can be proved with a tempdir and canned source file. Move live-scan to tests/ or ignore.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Unit test uses tempdir with fixed content; live-scan variant is moved to tests/ or ignored
- [x] #2 New test is deterministic wrt tokei version and file contents
<!-- AC:END -->
