---
id: TASK-008
title: "ProgressDisplay::on_plan_started exceeds 50-line function threshold"
status: To Do
assignee: []
created_date: '2026-04-07 00:00:00'
labels: [rust-code-quality, CQ, FN-1, low, effort-S, crate-runner]
dependencies: []
---

## Description

**Location**: `crates/runner/src/display.rs:268-340`
**Anchor**: `fn on_plan_started`
**Impact**: At 72 lines, this function exceeds the FN-1 threshold of 50 lines. It operates at three abstraction levels: (1) mapping command IDs to display names (268-278), (2) rendering header lines and creating pending progress bars (280-311), and (3) setting up footer separator and progress bar (316-339).

**Notes**:
The function has low nesting (max 2 levels) and each section is clearly separated, so cognitive load is moderate. Extract two helpers to bring it under threshold:

```rust
fn create_pending_bars(&mut self, pending_lines: &[String]) { ... }
fn create_footer(&mut self) { ... }
```

This would reduce `on_plan_started` to ~25 lines of orchestration. The footer creation logic also partially duplicates the fallback branch in `on_run_finished` (lines 491-517), so extracting `create_footer` could serve both call sites.
