---
id: TASK-1185
title: >-
  ARCH-1: stack.rs spans 280 production lines mixing Stack enum, detect,
  embedded TOML, defaults
status: To Do
assignee:
  - TASK-1264
created_date: '2026-05-08 08:11'
updated_date: '2026-05-08 13:18'
labels:
  - code-review-rust
  - arch
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/stack.rs:38`

**What**: Stack defines the enum, manifest-extension probes, ancestor walk with canonicalize fallback, ACCEPTED_NAMES, MAX_DETECT_DEPTH, the embedded-TOML metadata table, and default_commands parsing — eight separable concerns wedged into one impl block plus 540 lines of tests in the same file.

**Why it matters**: Adding stack-specific detection logic means editing the same file as the embedded TOML loader and the symlink-canonicalize walk. Mirrors TASK-1147 (run-before-commit/lib.rs 702 lines) and TASK-1121 (deps parse.rs 748 lines).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 stack.rs is split into at least two modules (e.g. stack/mod.rs enum + stack/detect.rs walk + stack/metadata.rs embedded-TOML table) so adding a new stack touches only metadata.
- [ ] #2 Public API (Stack::resolve, Stack::detect, Stack::default_commands, Stack::manifest_files, Stack::ACCEPTED_NAMES) preserved; existing tests pass unchanged.
<!-- AC:END -->
