---
id: TASK-0086
title: >-
  DUP-1: hook tests duplicate install_hook/ensure_config assertions across 3
  files
status: Done
assignee: []
created_date: '2026-04-17 11:33'
updated_date: '2026-04-17 15:05'
labels:
  - rust-codereview
  - dup
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:142`

**What**: run-before-commit and run-before-push test suites reimplement the same test bodies that already exist in hook-common tests with only string substitutions.

**Why it matters**: ~400 LOC of near-identical test code; any change in install_hook semantics requires edits in 3 test suites.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Remove the per-hook install_hook/ensure_config tests and keep only hook-common parameterized tests
- [ ] #2 Retain only wrapper-specific tests (HOOK_SCRIPT content, should_skip env var name) in each hook crate
<!-- AC:END -->
