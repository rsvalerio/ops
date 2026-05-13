---
id: TASK-1394
title: >-
  FN-2: atomic_write spans 166 lines mixing path resolution, permission
  preservation, and cross-platform write/cleanup
status: Done
assignee:
  - TASK-1450
created_date: '2026-05-13 18:06'
updated_date: '2026-05-13 19:13'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/edit.rs:108`

**What**: `atomic_write` is ~166 lines (108-274) and bundles parent resolution, capturing existing permissions, the cfg(unix) permission preservation branch, atomic rename, and tempfile cleanup-on-error. Each concern is independently testable and would benefit from extraction.

**Why it matters**: FN-2 flags functions >50 lines. Long, multi-branch I/O functions are hard to fully cover with unit tests and tempting to grow further. Splitting into a few helpers improves readability and reduces the surface area each change must reason about.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract at least two helpers (e.g. resolve_parent_and_filename, preserve_or_default_mode) so atomic_write is under 80 lines
- [ ] #2 All existing edit.rs tests pass without modification
<!-- AC:END -->
