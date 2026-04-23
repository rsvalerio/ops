---
id: TASK-0229
title: 'ERR-4: run_about_deps swallows warm-up provider errors with let _ ='
status: Done
assignee: []
created_date: '2026-04-23 06:33'
updated_date: '2026-04-23 08:56'
labels:
  - rust-code-review
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/deps.rs:20`

**What**: Same pattern: duckdb and metadata warm-up errors silently dropped.

**Why it matters**: Same observability gap for deps subpage.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Log on Err
- [x] #2 Discriminate NotFound vs. real errors
<!-- AC:END -->
