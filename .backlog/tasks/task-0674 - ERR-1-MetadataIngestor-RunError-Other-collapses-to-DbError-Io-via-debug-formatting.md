---
id: TASK-0674
title: >-
  ERR-1: MetadataIngestor RunError::Other collapses to DbError::Io via debug
  formatting
status: Done
assignee:
  - TASK-0737
created_date: '2026-04-30 05:14'
updated_date: '2026-04-30 17:58'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/ingestor.rs:27-29`

**What**: Unhandled `RunError` variants are converted with `std::io::Error::other(format!("unexpected RunError variant: {other:?}"))` — debug formatting strips structured context and downgrades to a generic `DbError::Io`.

**Why it matters**: Loses subprocess exit code, signal info, and stderr; `is_manifest_missing`-style downstream classification of `io::ErrorKind::NotFound` will misclassify. Prefer an explicit `DbError::Other` (or extend `DbError` with `Subprocess`) carrying the source error.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Map each RunError variant explicitly; add a non-Io DbError arm for 'other' subprocess errors so kind-based classification stays correct
- [ ] #2 Preserve Display (not Debug) of the underlying error
<!-- AC:END -->
