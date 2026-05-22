---
id: TASK-1595
title: >-
  ERR-1: test-coverage collect_coverage discards stderr on the success path,
  hiding llvm-cov warnings and instrumentation skips
status: Done
assignee:
  - TASK-1634
created_date: '2026-05-21 22:52'
updated_date: '2026-05-22 08:28'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/parse.rs:256-282`

**What**: When `output.status.success()` is true, stderr is silently dropped. `cargo llvm-cov` can succeed while emitting warnings (compiler warnings, llvm-cov diagnostics, "no tests found in target X"). The soft-fail branch surfaces stderr via `format_error_tail`, but the success branch emits nothing — not even at `debug!`/`trace!`.

**Why it matters**: Coverage runs use a 15-minute timeout. When a target silently drops out of instrumentation the only signal is "row missing from `coverage_files`", indistinguishable from "file never existed." The asymmetry with the soft-fail branch (which was deliberately built for diagnosability) leaves a blind spot on the happy path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 On the success branch, emit tracing::debug! (or info! when stderr is non-empty) with stderr_tail produced via format_error_tail(&output.stderr, 5)
- [ ] #2 Add a unit test driving a synthetic Output { status: success, stderr: warning bytes } through a thin extracted helper and asserts the diagnostic field is captured
- [ ] #3 Document the contract on collect_coverage so a future contributor does not strip the breadcrumb as noise
<!-- AC:END -->
