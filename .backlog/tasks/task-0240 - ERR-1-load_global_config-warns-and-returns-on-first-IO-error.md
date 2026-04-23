---
id: TASK-0240
title: 'ERR-1: load_global_config warns-and-returns on first IO error'
status: To Do
assignee: []
created_date: '2026-04-23 06:35'
updated_date: '2026-04-23 06:46'
labels:
  - rust-code-review
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader.rs:144`

**What**: On a non-NotFound error from read_config_file(path), function warns and returns without trying the second candidate path.

**Why it matters**: Corrupted global config is silently ignored; user thinks config is applied.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Propagate error with path context
- [ ] #2 Add explicit test for permission-denied/parse-error global config
<!-- AC:END -->
