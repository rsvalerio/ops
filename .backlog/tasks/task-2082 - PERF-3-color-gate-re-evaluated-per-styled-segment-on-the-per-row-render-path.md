---
id: TASK-2082
title: 'PERF-3: color gate re-evaluated per styled segment on the per-row render path'
status: To Do
assignee:
  - TASK-2244
created_date: '2026-09-07 22:58'
updated_date: '2026-09-08 10:58'
labels:
  - code-review-rust
  - performance
dependencies: []
modified_files:
  - crates/theme/src/style/sgr.rs
  - crates/theme/src/configurable.rs
priority: medium
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/style/sgr.rs:109`

**What**: `apply_with_prefix` calls `color_enabled()` on every invocation, and `color_enabled()` resolves `ops_core::style::no_color_env()` — a live `std::env::var_os("NO_COLOR")` read (global env lock + scan) — every time. `ConfigurableTheme::render_slot` (`crates/theme/src/configurable.rs:308`) invokes `apply_with_prefix` three to four times per rendered row (label, separator, trailing), so every step line and every report row pays three to four env-var reads. The TTY half is already cached in a `OnceLock` in ops-core; only the env half is re-read, deliberately, so `EnvGuard` tests can flip `NO_COLOR` at runtime (documented on `apply_style`).

**Why it matters**: this crate's own history sets the bar — TASK-0747 precomputed SGR prefixes, TASK-0746 and TASK-1130 each removed a single intermediate `String` allocation from this exact path, and TASK-1975 hoisted `icon_column_width` out of the per-row loop. A locked env lookup per styled segment is heavier than any of those, and the fix is subtractive (hoist a boolean) rather than additive complexity. Resolving the gate once per public render entry point (`render_slot`, `render_summary_text`, `render_report`, `render_error_detail`) and threading it through the private helpers preserves the `EnvGuard` test seam, the same injection pattern `render_error_block_gated` already established (TEST-25 / TASK-1979).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The NO_COLOR env var is read at most once per public render entry point, not once per apply_with_prefix call
- [ ] #2 EnvGuard-based tests (step_line_is_plain_when_stderr_is_redirected, label_color_does_not_affect_non_tty_output, summary_color_does_not_affect_non_tty_output) still pass without modification
- [ ] #3 Rendered output is byte-identical before and after (existing 127-test suite green)
<!-- AC:END -->
