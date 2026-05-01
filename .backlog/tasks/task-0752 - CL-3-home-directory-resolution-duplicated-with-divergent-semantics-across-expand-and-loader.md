---
id: TASK-0752
title: >-
  CL-3: home directory resolution duplicated with divergent semantics across
  expand and loader
status: Triage
assignee: []
created_date: '2026-05-01 05:53'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/expand.rs:105-109` and `crates/core/src/config/loader.rs:210-226`

**What**: Variables::try_expand resolves ~ via inline closure trying HOME then USERPROFILE. global_config_path encodes a different platform-aware order (XDG_CONFIG_HOME, then per-OS branch on cfg!(windows)). Two helpers answer "where is the user home?" with divergent semantics.

**Why it matters**: CL-3 / ARCH-4: shared platform invariants should live in one place. Future Windows-native handling will only mirror one site, producing user-visible drift.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract a single home-dir helper (e.g. crates/core/src/paths.rs::home_dir())
- [ ] #2 Document resolution order and cfg!(windows) branching in one place
- [ ] #3 Both call sites delegate to the new helper
<!-- AC:END -->
