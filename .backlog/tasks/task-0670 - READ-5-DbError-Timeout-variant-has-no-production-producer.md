---
id: TASK-0670
title: 'READ-5: DbError::Timeout variant has no production producer'
status: Done
assignee:
  - TASK-0738
created_date: '2026-04-30 05:14'
updated_date: '2026-04-30 18:31'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/error.rs:39-46`

**What**: `DbError::Timeout { label, timeout_secs: u64 }` exists in the type but no production code path actually constructs it — only tests reference timeouts (the run-before-commit timeout uses its own HasStagedFilesError::Timeout). The enum carries `#[allow(dead_code)]` which hides this.

**Why it matters**: Confusing for readers to encounter a variant with no producer and no plan documented. Either a TODO/issue comment should explain who will produce it, or the variant should be removed until needed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add a doc comment naming the planned producer (or a tracking task), or remove the variant
- [ ] #2 If kept, demonstrate at least one production constructor or a #[doc(hidden)] factory used by callers
<!-- AC:END -->
