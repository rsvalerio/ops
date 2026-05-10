---
id: TASK-0819
title: >-
  PERF-2: parse_package_json builds authors by .push after match instead of
  with_capacity
status: Done
assignee:
  - TASK-0823
created_date: '2026-05-01 06:03'
updated_date: '2026-05-01 09:21'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/package_json.rs:91-101`

**What**: authors Vec is Vec::new(); size is bounded by 1 + raw.contributors.len() and could be allocated once via Vec::with_capacity(1 + raw.contributors.len()).

**Why it matters**: Hot during About card rendering; small but trivially fixable, and consistent with PERF-2 elsewhere.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Allocate with with_capacity(1 + raw.contributors.len())
- [ ] #2 No behavioural change
<!-- AC:END -->
