---
id: TASK-0307
title: >-
  FN-1: install_hook spans ~58 lines and mixes canonicalize + create_new +
  legacy upgrade + bail formatting
status: Done
assignee:
  - TASK-0324
created_date: '2026-04-24 08:53'
updated_date: '2026-04-25 12:41'
labels:
  - rust-code-review
  - complexity
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: extensions/hook-common/src/install.rs:23-80

**What**: install_hook combines canonicalization, create_new write, read-existing, legacy detection, overwrite, and bail formatting at different abstraction levels in one function.

**Why it matters**: FN-1 threshold exceeded; each branch is hard to unit-test in isolation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Extract handle_existing_hook and write_new_hook helpers
- [x] #2 Top-level install_hook body ≤30 lines
<!-- AC:END -->
