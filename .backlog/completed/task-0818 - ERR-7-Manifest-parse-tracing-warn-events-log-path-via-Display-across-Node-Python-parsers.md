---
id: TASK-0818
title: >-
  ERR-7: Manifest-parse tracing::warn! events log path via Display across
  Node/Python parsers
status: Done
assignee:
  - TASK-0823
created_date: '2026-05-01 06:03'
updated_date: '2026-05-01 09:21'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/package_json.rs:81-89`, `extensions-python/about/src/lib.rs:175-181`, `extensions-python/about/src/units.rs:59`

**What**: All three crates emit path = %path.display() on parse failure. A workspace path containing a newline (legal on Unix) breaks log-line parsing the same way TASK-0665 documented.

**Why it matters**: Same class as TASK-0665 but in a fan-out the original task did not enumerate. Project-wide consistency: pick Display-with-sanitisation or Debug, apply uniformly.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Convert all path log fields in these three files to the project chosen sanitised form (Debug or escape)
- [ ] #2 Add a single shared helper if not already present in ops_core::log_fields
<!-- AC:END -->
