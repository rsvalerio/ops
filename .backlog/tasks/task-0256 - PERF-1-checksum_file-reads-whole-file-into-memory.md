---
id: TASK-0256
title: 'PERF-1: checksum_file reads whole file into memory'
status: Done
assignee: []
created_date: '2026-04-23 06:36'
updated_date: '2026-04-23 09:15'
labels:
  - rust-code-review
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:92`

**What**: std::fs::read allocates full buffer when streaming into Sha256 would suffice; coverage/tokei JSONs can be tens of MB.

**Why it matters**: Avoidable memory spike on large ingests.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Use BufReader + 64 KiB chunk loop into Sha256::update
- [x] #2 Equality test between streaming and in-memory checksum
<!-- AC:END -->
