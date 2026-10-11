---
id: TASK-2456
title: 'TEST-3: ops-theme''s 2000-line public-API test suite lives entirely inline in src/tests'
status: Done
assignee: []
created_date: '2026-10-10 15:24'
updated_date: '2026-10-10 21:29'
labels:
  - code-review-rust
  - tests
dependencies: []
parent_task_id: 'TASK-2609'
modified_files:
  - crates/theme/src/tests/mod.rs
  - crates/theme/src/lib.rs
priority: medium
ordinal: 1000
dedup_key: 'TEST-3:crates/theme/src/tests/mod.rs:mod tests'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/tests/mod.rs` (suite root), 13 submodule files, ~2043 lines total

**What**: The crate has no `tests/` directory; all 112 tests across 14 files sit in `#[cfg(test)] mod tests` under `src/tests/`. Twelve of the fourteen files exercise only the public API (render_basics, render_report, render_summary, boxed_layout, edge_case_width, unicode, deserialize, format_duration, left_pad, overhead_diagnostic, resolve, and most of render_basics' colour-gate checks). Only `error_block_color.rs` and `error_block_sanitise.rs` genuinely need private access (the crate-visible `render_error_block_gated`), plus one `color_enabled_for` call in render_basics.rs:139.

**Why it matters**: TEST-3 — the dividing line is the surface under test: anything that only touches the public API is an integration test and belongs in `tests/`, proving the API is usable from outside the crate. The separate-files layout mitigates the readability cost, but the public-surface tests still cannot fail to compile when an internal detail is repackaged, which is the protection `tests/` provides.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Public-API-only test files move to crates/theme/tests/ as integration tests; files needing private access (error_block_color, error_block_sanitise) may remain inline with a comment saying why
- [x] #2 cargo test passes with the suite in its new location and no test logic is rewritten beyond import paths

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Suite moved to crates/theme/tests/public_api/ (one integration binary, shared fixture in main.rs). error_block_color/error_block_sanitise remain inline (private render_error_block_gated). One substitution: render_basics' duplicated private-gate assertion color_enabled_for(false,false) was dropped from the moved copy — that exact assertion stays covered by the inline style::sgr test gate_ignores_stdout_and_follows_stderr; also snap() made const fn and the two u64::MAX-as-f64 saturation tests' scoped allow extended to cast_precision_loss (mirrors the lib.rs test-cfg allowance the inline suite had). Orphaned #[cfg(test)] re-export of color_enabled_for removed from style.rs.
<!-- SECTION:NOTES:END -->
