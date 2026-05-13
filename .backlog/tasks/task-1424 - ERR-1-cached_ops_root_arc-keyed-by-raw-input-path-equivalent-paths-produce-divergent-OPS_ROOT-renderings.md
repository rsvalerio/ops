---
id: TASK-1424
title: >-
  ERR-1: cached_ops_root_arc keyed by raw input path; equivalent paths produce
  divergent OPS_ROOT renderings
status: To Do
assignee:
  - TASK-1455
created_date: '2026-05-13 18:22'
updated_date: '2026-05-13 19:09'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/expand.rs:120`

**What**: The cache HashMap key is `ops_root.to_path_buf()` verbatim. Two semantically equal roots (`./project` vs `/abs/project`, symlinked vs canonical) produce distinct entries and distinct `OPS_ROOT` renderings, which then leak through argv/env expansion.

**Why it matters**: Operators who derive the workspace root twice in one process (e.g. relative for dry-run, absolute for exec) get inconsistent `$OPS_ROOT` substitution between the two paths. Combined with the unbounded cache (task-1418) this also amplifies leakage of synonym keys.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Document the canonicalisation contract at the call site, or canonicalise before insertion
- [ ] #2 Regression test: two equivalent paths (relative + absolute, symlinked) yield the same Arc<str>
<!-- AC:END -->
