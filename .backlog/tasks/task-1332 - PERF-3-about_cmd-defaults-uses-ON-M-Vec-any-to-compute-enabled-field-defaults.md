---
id: TASK-1332
title: >-
  PERF-3: about_cmd::defaults uses O(N*M) Vec::any to compute enabled-field
  defaults
status: Done
assignee:
  - TASK-1384
created_date: '2026-05-12 16:26'
updated_date: '2026-05-12 23:23'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/about_cmd.rs:53-61`

**What**: `defaults` iterates `about_fields` and for each candidate calls `currently_enabled.iter().any(|enabled| enabled == f.id)`, an O(N*M) scan. Building a `HashSet<&str>` of currently-enabled ids once and probing per candidate would be linear.

**Why it matters**: Path is hit when rendering About output / configuring fields; lists grow with extension count. Trivial fix with no semantic change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Lookup against currently_enabled is O(1) (set-backed).
- [ ] #2 Existing about_cmd tests pass without modification.
<!-- AC:END -->
