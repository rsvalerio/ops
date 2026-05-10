---
id: TASK-0689
title: >-
  ERR-4: go.work out-of-tree use members emit unit with path that LIKE-matches
  no tokei rows
status: Done
assignee:
  - TASK-0736
created_date: '2026-04-30 05:16'
updated_date: '2026-04-30 10:31'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/modules.rs:33-41`

**What**: When a `go.work` `use` directive points outside cwd (`../shared`), the warning message says "LOC stats will be empty" but the unit is still emitted with `path = "../shared"`, which `tokei_files.file LIKE 'path%'` will not match — and downstream consumers receive a unit with no LOC and no flag distinguishing it from a present-but-empty module.

**Why it matters**: An operator reading the About card sees "shared: 0 lines" and assumes the code is empty rather than out-of-tree. The diagnostic only surfaces in tracing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Either skip out-of-tree members entirely (with the existing warn) or annotate the emitted ProjectUnit.description with '(outside project root)' so the card differentiates from an empty module
<!-- AC:END -->
