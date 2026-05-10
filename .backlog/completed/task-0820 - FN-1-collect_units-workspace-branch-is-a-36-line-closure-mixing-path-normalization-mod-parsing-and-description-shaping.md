---
id: TASK-0820
title: >-
  FN-1: collect_units workspace branch is a 36-line closure mixing path
  normalization, mod parsing, and description shaping
status: Done
assignee:
  - TASK-0823
created_date: '2026-05-01 06:03'
updated_date: '2026-05-01 09:21'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/modules.rs:28-78`

**What**: The dirs.into_iter().map closure body inlines normalize_module_path, read_mod_info, out-of-tree detection with a tracing::warn, last-segment naming, description string composition, and ProjectUnit construction.

**Why it matters**: The function reads as orchestration but does the work itself. Extracting unit_from_use_dir(cwd, dir) -> ProjectUnit would mirror the symmetry already present in the Node units provider resolve_member_globs flow.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract per-dir construction into a named helper
- [ ] #2 collect_units body <=20 lines
<!-- AC:END -->
