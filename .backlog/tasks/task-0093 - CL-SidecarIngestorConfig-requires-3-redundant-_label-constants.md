---
id: TASK-0093
title: 'CL: SidecarIngestorConfig requires 3 redundant _label constants'
status: To Do
assignee: []
created_date: '2026-04-17 11:33'
updated_date: '2026-04-17 12:07'
labels:
  - rust-codereview
  - cl
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/ingestor.rs:32`

**What**: SidecarIngestorConfig requires callers to specify create_label, view_label, count_label as separate &static strs even though they are always formed as "{count_table} create|view|count".

**Why it matters**: Increases cognitive load and invites drift; derivable labels would eliminate 3 fields.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Derive labels from name/count_table inside load_with_sidecar using format!
- [ ] #2 Remove the three _label fields from SidecarIngestorConfig
<!-- AC:END -->
