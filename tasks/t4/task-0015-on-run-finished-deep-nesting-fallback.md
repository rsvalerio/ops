---
id: TASK-0015
title: "on_run_finished has 5-level nesting and fallback path doubles complexity"
status: Triage
assignee: []
created_date: '2026-04-09 19:25:00'
labels: [rust-code-quality, CQ, FN-2, CL-5, high, crate-runner]
dependencies: [TASK-0014]
---

## Description

**Location**: `crates/runner/src/display.rs:467-518`
**Anchor**: `fn on_run_finished`
**Impact**: Lines 501-506 reach 5 levels of nesting (function → else → if is_tty → if-let Some → body). The method has two entirely different code paths: the normal path (footer bar created in `on_plan_started`) and the fallback path (no plan was started). The fallback path has a `let pb = if let Some(...) { } else { }` followed by a 3-branch `if/else if/else if` chain for TTY vs non-TTY separator handling. Each separator write path is structurally different.

**Notes**:
Extract `render_summary_with_footer(&mut self, ...)` for the normal path and `render_summary_fallback(&mut self, ...)` for the fallback. The fallback likely handles a race or error condition — document when it triggers and whether it is reachable in normal operation.
