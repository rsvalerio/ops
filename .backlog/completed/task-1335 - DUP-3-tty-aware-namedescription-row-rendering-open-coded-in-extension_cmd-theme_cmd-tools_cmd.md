---
id: TASK-1335
title: >-
  DUP-3: tty-aware name+description row rendering open-coded in extension_cmd,
  theme_cmd, tools_cmd
status: Done
assignee:
  - TASK-1384
created_date: '2026-05-12 16:27'
updated_date: '2026-05-12 23:27'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Files**:
- `crates/cli/src/extension_cmd.rs:299-349`
- `crates/cli/src/theme_cmd.rs:80-114`
- `crates/cli/src/tools_cmd.rs:43-69`

**What**: All three list views open-code the same shape: `is_tty` branch, cyan colour applied to the name column, `pad_to_display_width` for alignment, then description. `pad_to_display_width` is already shared; the surrounding tty+colour decoration is not.

**Why it matters**: Three places to keep in sync when colour policy or row formatting changes. Single helper would centralise the policy and remove a near-identical match in each command.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A single helper renders (name, description, is_tty) rows with the project's colour policy.
- [ ] #2 The three list views call that helper instead of repeating the tty-branch inline.
<!-- AC:END -->
