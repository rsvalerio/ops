---
id: TASK-1311
title: >-
  DUP-1: BufWriter+MakeWriter test scaffold open-coded in 2 places after
  consolidation
status: To Do
assignee:
  - TASK-1387
created_date: '2026-05-11 19:58'
updated_date: '2026-05-12 22:16'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd/tests.rs:339-365` and `crates/cli/src/run_cmd/tests.rs:769-800`

**What**: The same `BufWriter` newtype + `MakeWriter` impl + `capture_*` helper is duplicated nearly verbatim between `log_step_results_tests` (lines 339-365) and `raw_warnings_tests` (lines 769-800); the second copy differs only by `Level::DEBUG` → `Level::WARN`. Meanwhile `crate::test_utils::capture_warnings` (test_utils.rs:114-145) holds a third nearly-identical copy and its doc-comment explicitly cites this consolidation effort ("open-coded the same BufWriter + MakeWriter scaffold (~17 lines each)"). Two callsites slipped through.

**Why it matters**: Per DUP-1 this is a 25-line identical block with one literal differing — exactly the red flag. The consolidation work in `test_utils.rs` already proves the pattern; the remaining duplicates undermine it.

<!-- scan confidence: verified by grep — three BufWriter+MakeWriter structs across test_utils.rs:125, run_cmd/tests.rs:342, run_cmd/tests.rs:772 -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Generalize crate::test_utils::capture_warnings into a capture_tracing(level, f) (or add capture_debug as a thin wrapper) and delete the two duplicate scaffolds in run_cmd/tests.rs
- [ ] #2 Existing capture_warnings callsites in registry/tests.rs continue to compile and pass without changes
<!-- AC:END -->
