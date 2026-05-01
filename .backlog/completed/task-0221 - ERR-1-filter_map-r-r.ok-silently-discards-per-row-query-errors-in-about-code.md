---
id: TASK-0221
title: >-
  ERR-1: filter_map(|r| r.ok()) silently discards per-row query errors in about
  code
status: Done
assignee: []
created_date: '2026-04-23 06:33'
updated_date: '2026-04-23 08:53'
labels:
  - rust-code-review
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/code.rs:54`

**What**: `rows.filter_map(|r| r.ok()).collect()` drops any Err rows from the tokei_files query without logging.

**Why it matters**: Partial/corrupt results are surfaced as if complete; debugging "missing languages" becomes impossible.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Log each Err via tracing before dropping
- [x] #2 Propagate as None if any row fails
<!-- AC:END -->
