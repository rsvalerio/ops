---
id: TASK-0002
title: Repeated ProgressStyle clone in display rendering loops
status: Done
assignee: []
created_date: '2026-04-10 07:15:00'
updated_date: '2026-04-11 10:06'
labels:
  - rust-idioms
  - EFF
  - PERF-3
  - low
  - crate-runner
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/runner/src/display.rs:314-315,335,342,369,463,526`
**Anchor**: `fn create_pending_bars`, `fn on_step_started`, `fn create_footer`, `fn on_step_failed`
**Impact**: `self.pending_style.clone()` and `self.running_style.clone()` are called repeatedly in loops that create or update progress bars. `ProgressStyle` is a relatively heavyweight struct. For typical command counts (<20 steps) this is not a performance concern, but it deviates from PERF-3 (avoid clone when borrow suffices).

**Notes**:
`indicatif::ProgressBar::with_style` and `set_style` take ownership of `ProgressStyle`, requiring the clone. This is an `indicatif` API constraint. If the style count is fixed (pending/running/success/fail), consider caching pre-built `ProgressBar` templates or using `Arc<ProgressStyle>` — though `indicatif` doesn't support shared styles natively. Low priority given typical step counts.
<!-- SECTION:DESCRIPTION:END -->
