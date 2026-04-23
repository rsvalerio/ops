---
id: TASK-0232
title: >-
  READ-1: collect_tokei builds Tokei with empty excluded list — includes
  target/, .git/
status: To Do
assignee: []
created_date: '2026-04-23 06:34'
updated_date: '2026-04-23 06:46'
labels:
  - rust-code-review
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/lib.rs:115`

**What**: `let excluded: &[&str] = &[];` — scans every directory including build artifacts.

**Why it matters**: Produces inflated LOC counts and slow scans on real projects; misleading about code output.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Exclude target, .git, node_modules, .venv by default
- [ ] #2 Add a test verifying exclusions
<!-- AC:END -->
