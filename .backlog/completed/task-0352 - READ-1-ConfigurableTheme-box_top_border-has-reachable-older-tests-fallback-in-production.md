---
id: TASK-0352
title: >-
  READ-1: ConfigurableTheme::box_top_border has reachable older tests fallback
  in production
status: Done
assignee:
  - TASK-0418
created_date: '2026-04-26 09:35'
updated_date: '2026-04-27 10:27'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/configurable.rs:171`

**What**: box_top_border branches on snap.command_ids.is_empty() and falls back to a "Running 2/5 · 1m2s" title with comment "Fallback for callers that do not provide command IDs (e.g. older tests)". BoxSnapshot::new defaults command_ids: &[].

**Why it matters**: A test-only branch reachable from production is exactly what READ-1/CL-3 flag: behaviour depends on whether the caller remembered an optional builder.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Make command_ids a required argument of BoxSnapshot::new and delete the empty-default and the fallback branch
- [ ] #2 Update affected tests to construct snapshots with explicit command_ids and verify only the production rendering path
<!-- AC:END -->
