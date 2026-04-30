---
id: TASK-0703
title: >-
  SEC-25: test-coverage load_coverage uses exists()-then-decide on
  coverage_files.json before delegating to ingestor
status: Done
assignee:
  - TASK-0738
created_date: '2026-04-30 05:28'
updated_date: '2026-04-30 18:36'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/lib.rs:281-289`

**What**: `load_coverage` does `if !json_path.exists() { bail }` and then calls `ingestor.load(data_dir, db)` which itself opens `coverage_files.json` via the sidecar pipeline. The `exists()` probe and the subsequent open are non-atomic — between them the file can disappear (concurrent `cleanup_artifacts`, antivirus scanner, mount remount), turning a clean "missing file" precondition error into a generic IO failure inside DuckDB.

**Why it matters**: Same SEC-25 anti-pattern flagged by TASK-0660 (Stack::detect) and TASK-0663 (write_workspace_sidecar): the safe shape is to attempt the open and let `ErrorKind::NotFound` drive the user-facing message. Today the function trades a clean precondition error for a TOCTOU window without gaining anything — the ingestor already reads the file.

**Why it matters**: silent-failure mode. The file existence is the only contract `load_coverage` enforces above the ingestor; a TOCTOU race produces a confusing DuckDB error rather than the intended "Run collect first" hint.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 load_coverage attempts the open atomically (or removes the existence probe) and surfaces a user-friendly NotFound message via the ingestor's own error
- [ ] #2 no other code path under extensions-rust/test-coverage uses exists()-then-decide on coverage_files.json
<!-- AC:END -->
