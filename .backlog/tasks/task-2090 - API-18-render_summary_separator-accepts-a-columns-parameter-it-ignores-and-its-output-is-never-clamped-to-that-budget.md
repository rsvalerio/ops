---
id: TASK-2090
title: 'API-18: render_summary_separator accepts a columns parameter it ignores, and its output is never clamped to that budget'
status: To Do
assignee: []
created_date: '2026-09-07 22:58'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - api-design
dependencies: []
parent_task_id: 'TASK-2246'
modified_files:
  - crates/theme/src/configurable.rs
priority: low
ordinal: 16000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/configurable.rs:161`

**What**: `ConfigurableTheme::render_summary_separator(&self, _columns: u16)` ignores its only parameter — the underscore name says so — yet both call sites outside the crate (`crates/runner/src/display.rs:458` and `crates/runner/src/display/finalize.rs:157`) pass a real terminal width, signalling an expectation the body does not meet. Unlike every other render path in this crate (documented truncation policy in `configurable.rs` module docs: "no rendered line may exceed the column budget it was given"), the returned separator is never clamped: a user-configured `summary_separator` wider than the terminal (e.g. a long run of `─` glyphs in `.ops.toml`) renders past the last column and wraps.

**Why it matters**: the parameter is a contract the function silently breaks; a caller reading the signature reasonably assumes width-aware behaviour, and the crate's own CL-3 budget invariant is the one render path that escapes it. Either honour the budget (`truncate_to_width`, like every sibling method) or drop the parameter so the signature stops promising what the body does not do.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either the separator is clamped to the given columns budget via truncate_to_width when columns > 0 (with a test pinning visible_width(result) <= columns for an over-long configured summary_separator), or the parameter is removed and both runner call sites updated
- [ ] #2 Existing tests render_basics.rs::classic_summary_separator_is_rail and compact_summary_separator_is_empty still pass
<!-- AC:END -->
