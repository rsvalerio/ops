---
id: TASK-1188
title: >-
  DUP-3: NO_COLOR + IsTerminal gate duplicated across core/style.rs and
  theme/style/sgr.rs
status: To Do
assignee:
  - TASK-1265
created_date: '2026-05-08 08:11'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - dup
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/style.rs:11`

**What**: `core/src/style.rs:color_enabled` and `theme/src/style/sgr.rs:color_enabled` each define their own OnceLock<bool> cached `is_terminal()` plus identical `no_color_env()` helper. The doc comment on the core copy explicitly says "Mirrors the gating in theme::style::sgr::color_enabled".

**Why it matters**: Two OnceLock caches read different streams (stdout vs stderr) by accident of where they were authored, so a terminal where stdout is a TTY but stderr is piped (or vice versa) silently disagrees on color enablement between the two helpers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 One shared color_enabled resolver lives in a neutral location (e.g. ops_core::style) and theme::style::sgr calls it; the duplicate OnceLock<bool> is removed.
- [ ] #2 A test asserts both core::style::cyan and theme::style::sgr::apply_style agree on color enablement under the same NO_COLOR / TTY conditions.
<!-- AC:END -->
