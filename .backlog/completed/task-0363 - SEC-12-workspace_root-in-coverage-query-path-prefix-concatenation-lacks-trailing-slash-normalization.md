---
id: TASK-0363
title: >-
  SEC-12: workspace_root in coverage query path-prefix concatenation lacks
  trailing-slash normalization
status: Done
assignee:
  - TASK-0419
created_date: '2026-04-26 09:36'
updated_date: '2026-04-27 10:53'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/query/coverage.rs:77`

**What**: The dual-prefix join builds ? || / || m.path || /. If a caller passes workspace_root with a trailing /, the resulting prefix becomes /ws//crates/foo/ and absolute filenames fail to match silently.

**Why it matters**: Silent data loss: per-crate coverage drops to 0 with no diagnostic. Bound parameter so not an injection vector — correctness/UX issue.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Normalize workspace_root (strip trailing /) at validation time before using it as a join key
- [x] #2 Test asserts identical results for workspace_root with and without trailing slash
<!-- AC:END -->
