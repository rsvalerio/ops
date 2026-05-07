---
id: TASK-0951
title: 'SEC-33: read_workspace_sidecar reads sidecar with no byte cap'
status: Done
assignee:
  - TASK-1015
created_date: '2026-05-04 21:45'
updated_date: '2026-05-07 19:37'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:214` (read_workspace_sidecar)

**What**: `read_workspace_sidecar` calls `std::fs::read(&workspace_path)` with no `Read::take`-style cap. The sidecar lives under `<db>.ingest/<name>_workspace.txt`. Bytes are passed verbatim to `OsStr::from_encoded_bytes_unchecked` and persisted into `data_sources.workspace_root`.

**Why it matters**: A multi-GB or `/dev/zero`-symlinked sidecar OOMs the CLI before reaching the unsafe boundary. TASK-0910 (`.git/config`) and TASK-0831 (manifests) already capped sibling readers — this is the matching gap. Defense-in-depth even though the dir is 0o700 (TASK-0787).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 read_workspace_sidecar uses File::open + Read::take(cap+1) and bails on oversize input
- [x] #2 Cap aligned with MAX_GIT_CONFIG_BYTES / MAX_MANIFEST_BYTES (~4 MiB)
- [x] #3 Regression test plants an oversized sidecar and asserts the read errors instead of allocating
<!-- AC:END -->
