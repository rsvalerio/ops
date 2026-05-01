---
id: TASK-0787
title: >-
  SEC-25: data_dir create_dir_all uses default permissions; ingest dir may be
  world-writable
status: Triage
assignee: []
created_date: '2026-05-01 05:58'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:268`

**What**: provide_via_ingestor runs std::fs::create_dir_all(&data_dir)? on the ingest data directory (e.g. target/ops/data.duckdb.ingest) without setting restricted permissions. Default umask creates 0o755-ish dirs. The ingest dir holds workspace-root sidecar files and JSON sidecars whose contents the database trusts.

**Why it matters**: SEC-25/SEC-29 — secure defaults. target/ops is the de-facto trust boundary for the ingest pipeline. write_workspace_sidecar uses fsync/atomic-rename for durability but does nothing about ACLs. On multi-user systems the staged JSON could be tampered with between collect and load.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Set restrictive permissions (0o700 on Unix) when creating the ingest dir
- [ ] #2 Add a unix-only test asserting the mode after creation
<!-- AC:END -->
