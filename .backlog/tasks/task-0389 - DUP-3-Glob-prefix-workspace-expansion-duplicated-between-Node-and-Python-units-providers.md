---
id: TASK-0389
title: >-
  DUP-3: Glob-prefix workspace expansion duplicated between Node and Python
  units providers
status: To Do
assignee:
  - TASK-0417
created_date: '2026-04-26 09:40'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/units.rs:139` (also extensions-python/about/src/units.rs:72-96)

**What**: resolve_member_globs in extensions-node/about/src/units.rs:139-167 and extensions-python/about/src/units.rs:72-96 are structurally identical — split member on first *, read_dir(parent), keep entries whose dir contains a marker manifest. They differ only in marker file (package.json vs pyproject.toml).

**Why it matters**: Two near-clones of glob-resolution logic invite drift in correctness-sensitive behavior (symlink handling, empty-prefix * patterns, IO errors, .. traversal).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract a shared helper (e.g., ops_about::workspace::resolve_member_globs) that returns Vec<String>. Both providers call it, passing their marker filename
- [ ] #2 Add a unit test in the shared helper for the prefix/**/suffix form documented in the Node comment but not actually supported by the prefix-only match
<!-- AC:END -->
