---
id: TASK-0971
title: >-
  PERF-1: read_optional_text grows String via repeated reallocations instead of
  pre-sizing
status: Done
assignee: []
created_date: '2026-05-04 21:48'
updated_date: '2026-05-04 23:01'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/manifest_io.rs:56` (read_optional_text)

**What**: `read_optional_text` uses `String::new()` + `read_to_string` up to MAX_MANIFEST_BYTES, paying the doubling-resize cost on every manifest read. Hot path: each about-units render across a node/python/go workspace touches every member's manifest.

**Why it matters**: Trivial pre-size via file metadata (clamped to MAX_MANIFEST_BYTES). SEC-33 cap stays in force.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Pre-size the String via file metadata, clamped to MAX_MANIFEST_BYTES, before read_to_string
- [ ] #2 Oversize bail-out test still passes; metadata-known and metadata-unknown branches both covered
<!-- AC:END -->
