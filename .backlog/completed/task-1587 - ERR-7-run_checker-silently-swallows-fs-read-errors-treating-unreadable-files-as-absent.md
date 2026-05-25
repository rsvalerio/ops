---
id: TASK-1587
title: >-
  ERR-7: run_checker silently swallows fs::read errors, treating unreadable
  files as absent
status: Done
assignee:
  - TASK-1636
created_date: '2026-05-21 22:46'
updated_date: '2026-05-22 12:17'
labels:
  - code-review-rust
  - err
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/lib.rs:147-150`

**What**: Inside `run_checker`, files whose `std::fs::read` fails (permission denied, mid-walk deletion, IO error, etc.) are silently skipped via `Err(_) => continue`. The error is discarded — the file is neither counted in `files_scanned` nor reported in `files_failed`, and no diagnostic reaches the writer.

**Why it matters**: `check-json`/`check-yaml` are gating tools (the CLI exits non-zero on `failed()`). If a file with broken permissions or transient IO error is silently ignored, the gate passes even though the file was never validated. A pre-commit hook that fails open on read errors gives a false sense of safety, especially in CI where a permission-mode mismatch could silently disable validation across the whole repo.

**Suggested fix**: Either treat the IO error as a check failure (push into `files_failed` with the OS error message and increment `files_scanned`) or at minimum emit a diagnostic line to `writer`. Match the pattern used elsewhere in the workspace's text-fixers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 IO read errors during run_checker surface to the user (either via files_failed or a written diagnostic)
- [x] #2 A test covers the read-error path (e.g. unreadable file via permissions or simulated IO error) and asserts the failure is reported
<!-- AC:END -->
