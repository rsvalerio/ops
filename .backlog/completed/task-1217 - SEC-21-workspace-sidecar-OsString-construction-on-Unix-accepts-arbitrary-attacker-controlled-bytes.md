---
id: TASK-1217
title: >-
  SEC-21: workspace sidecar OsString construction on Unix accepts arbitrary
  attacker-controlled bytes
status: Done
assignee:
  - TASK-1259
created_date: '2026-05-08 08:20'
updated_date: '2026-05-08 13:37'
labels:
  - code-review-rust
  - sec
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:273-287`

**What**: UNSAFE-1 / TASK-1104 hardened read_workspace_sidecar to avoid OsStr::from_encoded_bytes_unchecked's UB by switching to safe OsString::from_vec on Unix. The result is correct (no UB) but undisciplined: any byte sequence — including embedded \\n, NUL, ANSI escape, or path-traversal segments — round-trips into the OsString and is later passed to upsert_data_source (validated for UTF-8 there) and Path::display consumers. Although the ingest dir is 0o700 (TASK-0787), an attacker who passes the symlink-blind read still gets to seed arbitrary bytes.

**Why it matters**: TASK-1104 deliberately accepts bytes "matching the writer's as_encoded_bytes output for any path that round-trips" — but a tampered sidecar may not be a path the writer produced. A defense-in-depth filter for ASCII control bytes at the read boundary closes the gap. The Windows non-Unix branch already requires valid UTF-8.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 read_workspace_sidecar rejects sidecar bytes containing any byte in 0x00..=0x1f or 0x7f with DbError::Io(InvalidData), mirroring the SEC-2 / TASK-1102 control-byte gate on RedactedUrl::redact. Tests covering legitimate non-UTF-8 paths keep passing.
- [x] #2 A new test read_workspace_sidecar_rejects_embedded_newline writes a sidecar containing b'/ws/path\nfake/path' and asserts read_workspace_sidecar returns Err(DbError::Io(...)) with ErrorKind::InvalidData.
<!-- AC:END -->
