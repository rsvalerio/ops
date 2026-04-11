---
id: TASK-0014
title: "on_plan_started mixes 5 abstraction levels in 73 lines"
status: Triage
assignee: []
created_date: '2026-04-09 19:25:00'
labels: [rust-code-quality, CQ, FN-1, READ-1, high, crate-runner]
dependencies: []
---

## Description

**Location**: `crates/runner/src/display.rs:268-340`
**Anchor**: `fn on_plan_started`
**Impact**: This event handler performs 5 distinct operations in sequence: (1) resolves display labels from the display map, (2) renders and emits the plan header, (3) renders pending step lines into a `Vec<String>`, (4) creates and registers progress bars per step, (5) creates footer separator and footer bars. These are data transformation, output/IO, UI widget construction, state management, and layout — all in one method. The method name suggests "handle an event" but reads as "bootstrap the entire UI".

**Notes**:
Split into: `resolve_step_labels()`, `render_and_emit_header()`, `create_step_bars()`, `create_footer()`. Each helper is independently testable and operates at a single abstraction level.
