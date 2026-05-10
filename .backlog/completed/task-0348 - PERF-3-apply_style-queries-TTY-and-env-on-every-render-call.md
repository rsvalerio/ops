---
id: TASK-0348
title: 'PERF-3: apply_style queries TTY and env on every render call'
status: Done
assignee:
  - TASK-0418
created_date: '2026-04-26 09:34'
updated_date: '2026-04-27 10:24'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/style.rs:15`

**What**: apply_style calls std::io::stderr().is_terminal() and std::env::var_os("NO_COLOR") on every invocation. Invoked many times per rendered step line.

**Why it matters**: Rendering happens on every progress tick. Repeated is_terminal syscalls and env-var reads add measurable latency on slow terminals/CI and are unsafe to interleave with std::env::set_var in 2024 edition (UNSAFE-8).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Introduce a process-level cached gate (e.g. OnceLock<bool>) computed once from is_terminal() and NO_COLOR
- [ ] #2 Keep apply_style_gated for tests and document that the cached gate is intentionally read-once for production
<!-- AC:END -->
