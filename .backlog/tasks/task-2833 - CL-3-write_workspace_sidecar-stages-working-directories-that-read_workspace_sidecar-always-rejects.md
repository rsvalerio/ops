---
id: TASK-2833
title: 'CL-3: write_workspace_sidecar stages working directories that read_workspace_sidecar always rejects'
status: Done
assignee: []
created_date: '2026-11-18 18:50'
updated_date: '2026-11-18 21:47'
labels:
  - code-review-rust
  - CL
dependencies: []
parent_task_id: 'TASK-2853'
modified_files:
  - extensions/sqlite/src/sql/ingest/sidecar.rs
priority: low
ordinal: 1000
dedup_key: 'CL-3:extensions/sqlite/src/sql/ingest/sidecar.rs:write_workspace_sidecar'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/sqlite/src/sql/ingest/sidecar.rs:30-40` (`write_workspace_sidecar`) vs `:63-113` (`read_workspace_sidecar`)

**What**: The writer persists `working_directory.as_os_str().as_encoded_bytes()` with no validation. The reader rejects any ASCII control byte (`<= 0x1f` or `0x7f`) and, off Unix, any non-UTF-8 bytes. On Windows, `as_encoded_bytes` yields WTF-8 for unpaired surrogates, which `from_utf8` rejects. For a workspace path that contains such a byte, which is legal on Unix (e.g. a newline in a directory name), collect succeeds (an expensive tool run) and stages the sidecar, and load then fails deterministically with `InvalidData`. Because the table never gets populated, the next invocation repeats the whole collect and fails the same way. The two halves of one format disagree about what is valid.

**Why it matters**: A deterministic failure is detected only after the costly collect step, and it repeats on every run. The error names the sidecar, not the real cause (the workspace path).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 write_workspace_sidecar applies the same acceptance rules as read_workspace_sidecar and fails before staging, naming the workspace path, when the path would be rejected on read
- [x] #2 A test round-trips an accepted path and asserts a path with a control byte is refused at write time

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.30 (code-review-rust run, 2026-11-18).

TASK-2853: writer and reader share sidecar_bytes_problem; write_workspace_sidecar refuses with InvalidInput naming the path before staging. Fail-before-collect follow-up filed as TASK-2870.

<!-- SECTION:NOTES:END -->
