---
id: TASK-1212
title: >-
  PATTERN-1: parse_replace_directive accepts Windows backslash targets without
  scrubbing path-traversal segments
status: Done
assignee:
  - TASK-1270
created_date: '2026-05-08 08:19'
updated_date: '2026-05-10 16:59'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/go_mod.rs:98-107`

**What**: parse_replace_directive returns the raw target string verbatim once it matches one of ./, ../, .\\, ..\\, /, or is_windows_absolute. The returned string flows into local_replaces and is consumed by unit_from_use_dir-style logic via cwd.join(&normalized). Unlike the go.work use directive case, this helper performs no scrubbing of `..` cancellation patterns or empty / `.` segments inside the path body.

**Why it matters**: Operator-controlled go.mod, low impact, but the inconsistency with the broader SEC-14 scrub policy makes the next adversarial-fixture finding a regression that is hard to triage.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Local-replace targets either are validated to contain no Component::ParentDir past the leading prefix the matcher already accepts, OR are normalised through a scrub_path_segments-style filter so embedded .. segments are dropped.
- [ ] #2 New tests exercise (a) replace target with embedded path-traversal segments yields a sanitised target (or skipped with tracing::warn!), and (b) compute_module_count does not double-count an adversarial replace that the scrub eliminated.
<!-- AC:END -->
