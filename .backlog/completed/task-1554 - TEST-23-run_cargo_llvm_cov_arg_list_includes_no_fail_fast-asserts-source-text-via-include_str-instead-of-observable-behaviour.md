---
id: TASK-1554
title: >-
  TEST-23: run_cargo_llvm_cov_arg_list_includes_no_fail_fast asserts source text
  via include_str! instead of observable behaviour
status: Done
assignee:
  - TASK-1577
created_date: '2026-05-19 15:34'
updated_date: '2026-05-19 18:05'
labels:
  - code-review-rust
  - tests
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/tests.rs:384-396`

**What**: The TASK-1057 regression guard reads `src/lib.rs` with `include_str!` and greps the source string for the literal `\"--no-fail-fast\"` and `\"--json\"`:

```rust
let src = include_str!(\"lib.rs\");
let needle = \"\\\"--no-fail-fast\\\"\";
assert!(src.contains(needle), ...);
```

This tests the source code's text, not the program's behaviour. A contributor who reformats the args list across lines (e.g. one-flag-per-line via `rustfmt`), renames the constant, moves the args into a `const ARGS: &[&str]`, or wraps the call in a helper will fail this test even though the binary still ships `--no-fail-fast`. Conversely a contributor who silently swaps the call site to `cargo nextest` while leaving a stale `\"--no-fail-fast\"` literal in a comment will pass it.

**Why it matters**: The intent (pin the regression) is sound; the implementation pins the wrong thing. Either expose the argv (`pub(crate) const LLVM_COV_ARGS: &[&str]`) and assert against the slice, or assert via a fake `run_cargo`-style seam. Source-text tests rot silently and erode trust in the suite (TEST-23 brittleness, also TEST-1 in that the assertion does not exercise `run_cargo_llvm_cov` at all).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Regression guard for TASK-1057 no longer reads src/lib.rs via include_str!
- [ ] #2 Test asserts against an exposed const slice (or an argv-capture seam) covering both --no-fail-fast presence and --json being the final flag
- [ ] #3 Test name + comment continue to reference TASK-1057 so the provenance is preserved
<!-- AC:END -->
