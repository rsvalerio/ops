---
id: TASK-0472
title: 'ERR-1: parse_update_output drops verb-prefixed lines with debug-level logging'
status: To Do
assignee:
  - TASK-0533
created_date: '2026-04-28 05:47'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:111-118`

**What**: When parse_action_line returns None for a line that *does* start with a known verb (e.g. cargo changes the format to `Updating serde from v1 to v2`), the line is logged at tracing::debug! and dropped. debug is below the default log level, so the count headline silently regresses to 0.

**Why it matters**: ERR-1/ERR-2: schema drift in upstream tooling becomes silent data loss. Verb-prefixed misses are highly likely to indicate format change and warrant warn rather than debug.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Promote the "starts with known verb but did not parse" branch from debug to warn so format drift surfaces
- [ ] #2 Cover with a unit test feeding a hypothetical-future cargo-update line and asserting the warn fires (or at minimum the entry is not silently dropped without observable signal)
<!-- AC:END -->
