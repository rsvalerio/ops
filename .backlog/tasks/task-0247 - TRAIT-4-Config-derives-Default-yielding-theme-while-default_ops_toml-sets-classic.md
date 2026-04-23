---
id: TASK-0247
title: >-
  TRAIT-4: Config derives Default yielding theme="" while default_ops_toml()
  sets classic
status: To Do
assignee: []
created_date: '2026-04-23 06:35'
updated_date: '2026-04-23 06:46'
labels:
  - rust-code-review
  - trait
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/mod.rs:26`

**What**: #[derive(Default)] yields theme="" rather than canonical "classic" used in default_ops_toml(); two sources of truth.

**Why it matters**: Two sources of truth for default config can drift.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Hand-implement Default for Config to delegate to toml::from_str(default_ops_toml())
- [ ] #2 Or document that Config::default is for tests only
<!-- AC:END -->
