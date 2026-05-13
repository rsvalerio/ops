---
id: TASK-1416
title: >-
  READ-5: OutputConfig::resolve_columns re-probes terminal_size on every call
  without caching
status: To Do
assignee:
  - TASK-1453
created_date: '2026-05-13 18:17'
updated_date: '2026-05-13 19:09'
labels:
  - code-review-rust
  - read
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/mod.rs:378`

**What**: `OutputConfig::resolve_columns` calls `terminal_size::terminal_size()` every invocation when `columns == AUTO_COLUMNS`. The result depends on `ioctl(TIOCGWINSZ)` or a Windows console handle and is cached nowhere; step-line rendering and help/about cards call `resolve_columns()` per render. Terminal width is process-stable except for SIGWINCH, which `ops` does not handle.

**Why it matters**: Each render path issues one syscall to recompute a value that almost never changes during a single command's execution. Memoize behind a `OnceLock<u16>` mirroring the discipline in `style::color_enabled` and `subprocess::output_byte_cap`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 cache resolved column width behind a OnceLock
- [ ] #2 document that SIGWINCH within a single ops <cmd> is not observed (matches TMPDIR_DISPLAY lifetime contract)
- [ ] #3 test asserts repeat resolve_columns calls return identical value without re-issuing terminal-probe
<!-- AC:END -->
