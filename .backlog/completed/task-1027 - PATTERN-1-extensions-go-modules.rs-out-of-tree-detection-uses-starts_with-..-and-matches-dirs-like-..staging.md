---
id: TASK-1027
title: >-
  PATTERN-1: extensions-go modules.rs out-of-tree detection uses
  starts_with("..") and matches dirs like ..staging
status: Done
assignee: []
created_date: '2026-05-07 20:23'
updated_date: '2026-05-07 23:11'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/modules.rs:55-69`

**What**: `unit_from_use_dir` checks `let out_of_tree = normalized.starts_with(\"..\");`. This is a substring match on the *string*, not a path-component match. It correctly catches `..` and `../shared` but also incorrectly flags any legal directory whose first component begins with two dots: `..staging/api`, `..hidden`, `..backup-2025` — all of which are valid POSIX directory names.

Such a unit is then:
- logged with `tracing::warn!("…points outside the project root…")` on every About run, polluting the operator's log,
- annotated with `(outside project root)` in the description,
- reported with zero LOC even though the unit lives entirely under cwd.

The correct test is on the first path component: split `normalized` on `/` (and `\\` for Windows-shaped paths in `go.work`) and check whether the first component equals exactly `..`. `Path::components` would also do, but the existing code already operates on the trimmed-but-still-string-shaped `normalized`.

**Why it matters**: real-world hit rate is low — directories starting with `..` are unusual — but the bug shows up loud (warn log + visible card label) when it does. The fix is one line and matches the contract the surrounding code already documents (\"out of tree\" = \"escapes cwd\", not \"begins with two dots\").
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Detect out-of-tree by component (split on / and \\, check first component == '..') rather than starts_with("..")
- [ ] #2 Add a test: a  directive (legal directory beginning with ..) is treated as in-tree, no warn emitted, no '(outside project root)' suffix
<!-- AC:END -->
