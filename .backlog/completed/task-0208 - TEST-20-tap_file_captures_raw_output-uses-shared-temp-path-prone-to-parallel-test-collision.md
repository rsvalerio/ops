---
id: TASK-0208
title: >-
  TEST-20: tap_file_captures_raw_output uses shared temp path prone to
  parallel-test collision
status: Done
assignee: []
created_date: '2026-04-23 06:32'
updated_date: '2026-04-23 14:59'
labels:
  - rust-code-review
  - test
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display/tests.rs:180`

**What**: Test hardcodes `std::env::temp_dir().join("ops_tap_test.log")` shared across parallel test processes.

**Why it matters**: Concurrent `cargo test` runs can overwrite each other's tap file, producing flaky failures when assertions race with another writer.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace std::env::temp_dir().join(...) with tempfile::NamedTempFile or tempfile::tempdir()
- [ ] #2 Verify two concurrent invocations pass under cargo test -- --test-threads=8
<!-- AC:END -->
