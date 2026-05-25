---
id: TASK-1085
title: >-
  PATTERN-1: extensions-go single-mod ProjectUnit uses empty path, matching
  every file via starts_with('')
status: Done
assignee: []
created_date: '2026-05-07 21:21'
updated_date: '2026-05-07 23:30'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/modules.rs:36-49`

**What**: For a non-workspace `go.mod`, `collect_units` constructs the unit with `String::new()` as path, with the comment "Empty path matches every file in tokei_files via starts_with." But `starts_with("")` matches every path in the table — including files not part of the Go module (vendored JS, generated artefacts under non-Go subdirs, etc.). LOC for the single-mod card therefore over-counts everything in cwd.

**Why it matters**: The other stacks (Node, Python) restrict per-unit LOC by the unit subpath. Go's single-mod path picks up the entire workspace tree, so the About card's "modules: 1 / N LOC" is inconsistent across stacks for the same project shape.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either filter tokei_files to the Go module root (e.g. .go files only) or document that the single-mod count is project-wide
- [ ] #2 Test pins behaviour against a project that contains unrelated non-Go files in cwd
- [ ] #3 Cross-stack invariant note in docstring
<!-- AC:END -->
