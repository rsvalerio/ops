---
id: TASK-0082
title: 'ERR-4: context(format!) is eager — should be with_context(|| format!)'
status: To Do
assignee: []
created_date: '2026-04-17 11:32'
updated_date: '2026-04-17 12:07'
labels:
  - rust-codereview
  - err
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/query.rs:34`

**What**: Multiple sites use .context(format!("...{label}")) eagerly allocating strings on every call, instead of .with_context(|| format!(...)).

**Why it matters**: Wasted allocations on the happy path; idiomatic consistency matters (with_context is used correctly in ingest.rs).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Convert all .context(format!(...)) to .with_context(|| format!(...)) in query.rs
- [ ] #2 Grep the extensions tree to enforce this pattern
<!-- AC:END -->
