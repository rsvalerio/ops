---
id: TASK-0152
title: 'UNSAFE-8: text_util tests mutate COLUMNS via set_var/remove_var'
status: To Do
assignee: []
created_date: '2026-04-22 21:22'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - UNSAFE
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/text_util.rs:208-216`

**What**: `get_terminal_width_default` calls `std::env::remove_var("COLUMNS")` and `std::env::set_var("COLUMNS", v)` to drive the test. In Rust 2024 these are `unsafe`, and they mutate process-global state shared by all parallel tests.

**Why it matters**: UNSAFE-8 + TEST-18. Other threads may observe the cleared `COLUMNS` and produce flaky widths; any parallel test reading COLUMNS is susceptible. Fix by injecting the terminal width (dependency injection) or gating with a serialized env guard.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 get_terminal_width_default no longer mutates process env, or uses a serial env guard
- [ ] #2 production get_terminal_width is refactored to accept a source callable for testability
<!-- AC:END -->
