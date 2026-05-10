---
id: TASK-0678
title: 'PERF-3: coverage_provider clones workspace.members vec twice per provide()'
status: Done
assignee:
  - TASK-0741
created_date: '2026-04-30 05:14'
updated_date: '2026-04-30 19:33'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/coverage_provider.rs:55-72`

**What**: `manifest.workspace.…members.clone()` allocates a `Vec<String>`, then `members.iter().map(String::as_str).collect()` allocates another Vec for the borrow view, even though `manifest` is already an `Arc<CargoToml>`.

**Why it matters**: Identity, units, and coverage providers all run on a single `ops about`. The Arc cache exists to avoid this; cloning the members vec defeats it on the hot path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Borrow members directly from manifest.workspace.as_ref() and pass &[String] slices through to the query helper
<!-- AC:END -->
