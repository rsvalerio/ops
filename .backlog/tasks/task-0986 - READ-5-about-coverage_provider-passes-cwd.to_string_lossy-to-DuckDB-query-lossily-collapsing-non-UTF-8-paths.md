---
id: TASK-0986
title: >-
  READ-5: about::coverage_provider passes cwd.to_string_lossy() to DuckDB query,
  lossily collapsing non-UTF-8 paths
status: Triage
assignee: []
created_date: '2026-05-04 21:59'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/coverage_provider.rs:68`

**What**: `query_crate_coverage(db, &member_strs, &cwd.to_string_lossy())` pipes the workspace `cwd` into a SQL query through `to_string_lossy`. On non-UTF-8 paths (Linux invalid UTF-8 bytes, NTFS WTF-16 down-conversions) the lossy conversion replaces invalid bytes with U+FFFD, so two distinct workspace paths collapse to the same query key and the coverage join silently mismatches per-member rows or returns rows scoped to an unrelated workspace. Sister case to TASK-0946, which fixed the same `to_string_lossy` lossy-collapse anti-pattern for workspace member relpaths in `query.rs`.

**Why it matters**: `ops about` claims to handle arbitrary cloned repos. A user whose checkout sits below a non-UTF-8 path (e.g. macOS NFD-vs-NFC corner cases on imported-from-Windows volumes) silently sees wrong per-crate coverage with no diagnostic, undermining the about page's health-signal contract.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Non-UTF-8 cwd either short-circuits query_crate_coverage with a tracing::warn breadcrumb (matching the TASK-0946 policy) or routes through an OsStr-based query path that does not require UTF-8
- [ ] #2 Regression test against a path containing a non-UTF-8 byte verifies the chosen behaviour and that no U+FFFD replacement key is sent to the query
<!-- AC:END -->
