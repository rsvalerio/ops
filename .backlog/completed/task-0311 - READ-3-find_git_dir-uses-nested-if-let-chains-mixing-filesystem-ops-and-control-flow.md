---
id: TASK-0311
title: >-
  READ-3: find_git_dir uses nested if-let chains mixing filesystem ops and
  control flow
status: Done
assignee:
  - TASK-0325
created_date: '2026-04-24 08:53'
updated_date: '2026-04-25 12:51'
labels:
  - rust-code-review
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: extensions/hook-common/src/git.rs:30-51

**What**: Three-branch nested if-let inside a for-loop mixes control flow with filesystem ops.

**Why it matters**: Harder to scan than flat match; the FS branching is obscured by indentation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Flatten with let-else and a single match on meta.file_type()
- [ ] #2 Body ≤20 lines; behavior unchanged by existing tests
<!-- AC:END -->
