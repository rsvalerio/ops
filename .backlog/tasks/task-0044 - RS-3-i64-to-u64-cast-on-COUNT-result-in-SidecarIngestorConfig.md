---
id: TASK-0044
title: 'RS-3: i64-to-u64 cast on COUNT(*) result in SidecarIngestorConfig'
status: Done
assignee: []
created_date: '2026-04-14 20:16'
updated_date: '2026-04-15 09:56'
labels:
  - rust-security
  - defense-in-depth
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
In extensions/duckdb/src/ingestor.rs:82, a DuckDB COUNT(*) result (i64) is cast to u64 via 'as u64'. While COUNT(*) cannot return negative values in practice, the 'as' cast would silently wrap a negative i64 to a large u64 in release builds (SEC-15). Defense-in-depth: use try_from or .max(0) as u64 to make the conversion explicitly safe. Affected crate: ops-duckdb. OWASP: A04 (Insecure Design). SEC-15.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 i64-to-u64 conversion in SidecarIngestorConfig::load_with_sidecar uses checked arithmetic (try_from or explicit clamp) instead of bare 'as' cast
<!-- AC:END -->
