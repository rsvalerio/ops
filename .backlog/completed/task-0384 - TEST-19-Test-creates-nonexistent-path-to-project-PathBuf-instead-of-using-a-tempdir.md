---
id: TASK-0384
title: >-
  TEST-19: Test creates /nonexistent/path/to/project PathBuf instead of using a
  tempdir
status: Done
assignee:
  - TASK-0421
created_date: '2026-04-26 09:39'
updated_date: '2026-04-27 16:12'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/ingestor.rs:97` (also test-coverage/src/ingestor.rs:54)

**What**: Two tests hardcode /nonexistent/path/to/project. While unlikely, this path could exist on developer machines or CI images and turn the negative test into a false negative.

**Why it matters**: Tests should not depend on the absence of an arbitrary absolute path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Replace with tempfile::tempdir() whose path is then deleted before use, or a known-impossible path like tempdir.path().join(does-not-exist)
- [x] #2 Tests still assert the error case
<!-- AC:END -->
