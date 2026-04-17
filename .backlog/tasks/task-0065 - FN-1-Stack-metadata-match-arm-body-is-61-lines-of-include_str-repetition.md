---
id: TASK-0065
title: 'FN-1: Stack::metadata match-arm body is 61 lines of include_str! repetition'
status: To Do
assignee: []
created_date: '2026-04-17 11:30'
updated_date: '2026-04-17 12:07'
labels:
  - rust-codereview
  - fn
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/stack.rs:83`

**What**: Each arm repeats `include_str!(concat!(env!(CARGO_MANIFEST_DIR), ...))` boilerplate for 9 stacks.

**Why it matters**: Adding a new stack repeats the macro incantation; risk of typos and duplicated boilerplate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Introduce a macro_rules! or helper that takes (manifest_files, toml_filename) and builds the tuple
- [ ] #2 Each arm reduces to a one-line declaration per stack
<!-- AC:END -->
