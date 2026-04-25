---
id: TASK-0312
title: >-
  DUP-3: HookConfig test fixtures duplicated across hook-common lib.rs,
  install.rs, config.rs
status: Done
assignee:
  - TASK-0324
created_date: '2026-04-24 08:53'
updated_date: '2026-04-25 12:41'
labels:
  - rust-code-review
  - duplication
  - test
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: extensions/hook-common/src/lib.rs:138-150; install.rs:139-163; config.rs:72-96

**What**: Same commit_config / push_config HookConfig literal rebuilt in three test modules.

**Why it matters**: Drift risk when HookConfig fields evolve; violates DUP-3.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Shared #[cfg(test)] pub(crate) mod fixtures module added
- [x] #2 All three test modules import from it
<!-- AC:END -->
