---
id: TASK-1429
title: >-
  DUP-3: BufWriter+MakeWriter tracing-capture scaffold duplicated across core
  test modules
status: Done
assignee:
  - TASK-1460
created_date: '2026-05-13 18:23'
updated_date: '2026-05-14 09:08'
labels:
  - code-review-rust
  - DUP
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/stack/mod.rs:194` (and `crates/core/src/config/merge.rs` collision-log test)

**What**: Multiple test functions in core define a local `BufWriter` + `MakeWriter` impl byte-for-byte. The CLI consolidated this into a shared `capture_tracing` helper (commit 0361a2c).

**Why it matters**: Drift risk; the consolidation that landed in cli should also land in core to keep the test scaffold single-sourced.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Move the BufWriter+MakeWriter helper into test_utils.rs (or a new shared test-support module)
- [ ] #2 All in-crate call sites adopt the shared helper; no per-test re-definitions remain
<!-- AC:END -->
