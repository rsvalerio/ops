---
id: TASK-1338
title: >-
  TEST-11: builtin_extensions_none_enabled_loads_all asserts only is_ok() —
  vacuous under no stack features
status: To Do
assignee:
  - TASK-1385
created_date: '2026-05-12 16:27'
updated_date: '2026-05-12 22:16'
labels:
  - code-review-rust
  - tests
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry/tests.rs:94-98`

**What**: Test name promises that when `enabled` is absent, all compiled-in extensions load — but the body asserts only `result.is_ok()` and drops the vec. With zero stack features compiled in, the vec is legitimately empty and the test always passes; with features compiled in, the test still verifies nothing. The adjacent `builtin_extensions_empty_enabled_list:87` asserts `exts.is_empty()`, so the precision is achievable.

**Why it matters**: Under default features the test asserts a vacuous property — closely related to TASK-1314. The "loads all" half of the contract is never exercised.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Under at least one stack-feature cfg gate, the test asserts !exts.is_empty() and a known extension name appears.
- [ ] #2 Under the no-stack-feature path, the test either skips or explicitly asserts exts.is_empty().
<!-- AC:END -->
