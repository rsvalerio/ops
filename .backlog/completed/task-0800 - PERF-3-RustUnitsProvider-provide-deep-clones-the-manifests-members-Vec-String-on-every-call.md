---
id: TASK-0800
title: >-
  PERF-3: RustUnitsProvider::provide deep-clones the manifest's members
  Vec<String> on every call
status: Done
assignee:
  - TASK-0822
created_date: '2026-05-01 06:01'
updated_date: '2026-05-01 07:00'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/units.rs:32-36`

**What**: members.clone() copies the whole Vec<String> of resolved member paths off the cached Arc<CargoToml> even though the closure that follows only needs to iterate them. Same shape TASK-0678 fixed for RustCoverageProvider.

**Why it matters**: Typed-manifest cache (TASK-0558) exists precisely so providers share an Arc<CargoToml> without copies; cloning out members defeats half of that win.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Borrow [String] from the cached manifest, sort a Vec<str> for ordering, and only to_string() when populating ProjectUnit path field
- [ ] #2 No clone of the source Vec<String> from the cached manifest
- [ ] #3 Behaviour preserved: same sort order, same final ProjectUnit values
<!-- AC:END -->
