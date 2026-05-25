---
id: TASK-1440
title: >-
  PERF-3: detect_terminal_width invokes terminal_size ioctl on every call
  without caching
status: Done
assignee:
  - TASK-1459
created_date: '2026-05-13 18:40'
updated_date: '2026-05-14 08:36'
labels:
  - code-review-rust
  - perf
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/output.rs:48-51`

**What**: `detect_terminal_width()` calls `terminal_size::terminal_size()` (an `ioctl(TIOCGWINSZ)` on Unix) on every invocation. Renderers call this from each step-line / about-card / table-sizing path, typically once per emitted row. Distinct from TASK-1416 (which targets `OutputConfig::resolve_columns` caching) — this is the lower-level probe itself.

**Why it matters**: Width can legitimately change on SIGWINCH, but per-row resolution is wasteful and existing column-resolution paths already assume a stable width per command run. Adds avoidable syscalls to every rendered line.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Cache the width behind a OnceLock per command invocation (or a Cell<Option<usize>> cleared on SIGWINCH); document that interactive resize during a single command is not observed mid-render
- [ ] #2 Test asserts repeated calls within one render produce one terminal_size invocation (via a feature-gated counter or probe-trait injection)
<!-- AC:END -->
