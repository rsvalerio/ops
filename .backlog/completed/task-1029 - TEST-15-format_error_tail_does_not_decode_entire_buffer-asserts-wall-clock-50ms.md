---
id: TASK-1029
title: >-
  TEST-15: format_error_tail_does_not_decode_entire_buffer asserts wall-clock <
  50ms
status: Done
assignee: []
created_date: '2026-05-07 20:23'
updated_date: '2026-05-08 06:24'
labels:
  - code-review-rust
  - test
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/output.rs:303-318`

**What**: The PERF-1 regression test for `format_error_tail` builds a ~4 MiB buffer, calls the function, then asserts `elapsed < std::time::Duration::from_millis(50)`. This is a wall-clock timing assertion in a unit test — exactly the anti-pattern TEST-15 flags. On a loaded CI runner (parallel `cargo test`, virtualised hosts, debug builds, slow disks, ASan/Miri runs) a 50 ms ceiling is easy to overrun even if the implementation never decodes the full buffer.

**Why it matters**: A flaky perf-regression gate erodes signal — the next time a real regression slips in, the failure looks like the usual flake and gets rerun-until-green. The PERF-1 contract should be expressed structurally (e.g. assert that `format_error_tail` never calls `from_utf8_lossy` on the full slice — measure via a counted reader, or assert the returned tail content + cap the buffer-walk to N newline scans), not via a wall-clock budget.

Candidate replacement: drive `format_error_tail` via a `&[u8]` slice whose length is enormous but whose decoded content is constant; assert the result is correct (already done) and additionally pin the byte-walk count via a counting `Read` adapter, or split the byte-walk into a separate testable helper.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Replace wall-clock < 50ms assertion with a structural check (e.g. counted byte-walk, decoded-segment count) that does not depend on host scheduling
- [x] #2 Existing correctness assertions on the tail content remain
- [x] #3 Test passes deterministically under `cargo test --release` and a single-threaded runner with --test-threads=1 on a loaded host
<!-- AC:END -->
