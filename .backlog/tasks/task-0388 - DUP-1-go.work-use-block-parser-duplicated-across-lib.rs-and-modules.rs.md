---
id: TASK-0388
title: 'DUP-1: go.work use-block parser duplicated across lib.rs and modules.rs'
status: Done
assignee:
  - TASK-0417
created_date: '2026-04-26 09:39'
updated_date: '2026-04-27 19:48'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/lib.rs:132` (also modules.rs:82-114)

**What**: parse_go_work in lib.rs:132-159 and workspace_use_dirs in modules.rs:82-114 both re-implement the same use (...) block parser for go.work, including the single-line use ./mymod form and comment skipping.

**Why it matters**: Already-observed drift: lib.rs filters comment lines; modules.rs filters them too — but the for_each_trimmed_line vs raw content.lines() choice differs and the trim-end-of-/ normalization only happens in modules.rs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract a single parse_go_work_use_dirs(root: &Path) -> Option<Vec<String>> into a shared module (e.g., extensions-go/about/src/go_work.rs) and call it from both providers
- [ ] #2 Both parse_go_work and workspace_use_dirs are deleted or become 1-line wrappers; existing tests in both files continue to pass
<!-- AC:END -->
