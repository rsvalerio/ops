---
id: TASK-2486
title: 'FN-1: parse_pyproject exceeds 50 lines, mixing manifest read, whole-doc parse, and field projection'
status: Done
assignee: []
created_date: '2026-10-10 15:30'
updated_date: '2026-10-10 22:15'
labels:
  - code-review
  - structure
dependencies: []
parent_task_id: 'TASK-2621'
modified_files:
  - extensions-python/about/src/lib.rs
priority: medium
ordinal: 1000
dedup_key: 'FN-1:extensions-python/about/src/lib.rs:parse_pyproject'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:240`

**What**: `parse_pyproject` spans lines 240-303 (~52 code lines excluding comments/blanks), over the FN-1 threshold of 50, and runs three jobs at different abstraction levels: (1) reading the cached manifest text, (2) whole-document `toml::from_str` with its degradation warn, (3) projecting eight `[project]` keys one by one plus the urls table walk (lines 269-300).

**Why it matters**: FN-1 asks each function to operate at a single abstraction level. The field-projection block alone is ~30 lines and is the natural extraction — a helper taking `&toml::Table` and `&Path` and returning the populated `Pyproject` fields — which would leave `parse_pyproject` as the read+parse scaffold it is described to be in its doc comment.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 No function in the crate exceeds 50 code lines, with parse_pyproject's field-projection block extracted to a named helper at one abstraction level
- [x] #2 cargo test -p ops-about-python passes unchanged (behaviour identical)

<!-- AC:END -->
