---
id: TASK-0276
title: 'TEST-5: print_categorized_help has no direct test for render insertion path'
status: Done
assignee: []
created_date: '2026-04-23 06:37'
updated_date: '2026-04-23 15:23'
labels:
  - rust-code-review
  - test
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/help.rs:147`

**What**: Helpers tested but full path (including `\nOptions:` insertion fallback) is not.

**Why it matters**: Regression in insertion logic silently fails.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Snapshot test capturing stdout
- [ ] #2 Cover long + short help forms
<!-- AC:END -->
