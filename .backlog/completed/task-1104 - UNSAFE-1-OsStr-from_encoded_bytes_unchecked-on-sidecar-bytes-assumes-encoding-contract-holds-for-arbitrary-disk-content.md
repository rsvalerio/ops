---
id: TASK-1104
title: >-
  UNSAFE-1: OsStr::from_encoded_bytes_unchecked on sidecar bytes assumes
  encoding contract holds for arbitrary disk content
status: Done
assignee: []
created_date: '2026-05-07 21:34'
updated_date: '2026-05-08 06:37'
labels:
  - code-review-rust
  - unsafe
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:272`

**What**: The SAFETY comment argues bytes "came from a prior `as_encoded_bytes` call", but the function's contract is over the bytes actually present on disk, not over what the writer intended. On Windows the encoding form is WTF-8 and arbitrary 4-MiB-capped bytes from a tampered or hand-edited sidecar can violate that invariant — undefined behaviour, not a corrupted-input issue. The 0o700 ingest dir hardens the typical case but does not establish soundness.

**Why it matters**: UB on adversarial/corrupted input; future port to non-Unix target inherits UB instead of an error.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Replace from_encoded_bytes_unchecked with safe construction: validate the bytes (OsStr::from_bytes on Unix, OsString::from(String::from_utf8(...)) elsewhere) or fall back to String::from_utf8_lossy and document the loss
- [x] #2 Add a Windows-targeted test that reads a sidecar containing bytes invalid as WTF-8 and asserts a typed DbError, not UB
- [x] #3 Drop the SAFETY comment's 'tampered hand-edited sidecar is corrupted-input issue, not soundness concern' claim once the unsafe is removed
<!-- AC:END -->
