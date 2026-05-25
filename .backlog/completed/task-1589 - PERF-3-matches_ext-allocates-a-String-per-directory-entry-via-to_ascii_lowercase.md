---
id: TASK-1589
title: >-
  PERF-3: matches_ext allocates a String per directory entry via
  to_ascii_lowercase
status: Done
assignee:
  - TASK-1636
created_date: '2026-05-21 22:46'
updated_date: '2026-05-22 12:17'
labels:
  - code-review-rust
  - perf
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/lib.rs:120-127`

**What**: `matches_ext` calls `e.to_ascii_lowercase()` (allocates a `String`) for every entry yielded by the discovery walk, then `allowed.iter().any(|a| *a == lower)` linear-scans the small allowed list. The allocation is unconditional even for files whose extension is already lowercase, and it occurs once per file in the entire repo walk.

**Why it matters**: This runs on every file in the candidate set, not just JSON/YAML files. On a large monorepo the allocator pressure shows up in the hot path of a pre-commit hook. The fix is a one-liner that avoids the allocation entirely: `allowed.iter().any(|a| a.eq_ignore_ascii_case(e))`.

**Severity**: Low — straightforward correctness-preserving optimization; symptomatic of forgetting `eq_ignore_ascii_case` exists.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 matches_ext does not allocate per-call (uses eq_ignore_ascii_case or equivalent)
- [x] #2 extension_matching_is_case_insensitive test continues to pass
<!-- AC:END -->
