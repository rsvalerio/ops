---
id: TASK-0497
title: >-
  READ-2: go.work parse_use_dirs does not strip inline // comments from use
  directive lines
status: Done
assignee:
  - TASK-0532
created_date: '2026-04-28 06:10'
updated_date: '2026-04-28 15:44'
labels:
  - code-review-rust
  - read
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/go_work.rs:34`

**What**: Inside a `use (` block, lines starting with `//` are skipped, but a line like `./api // legacy` is pushed verbatim including the comment. The same happens for single-line `use ./api // comment`. Down-stream, normalize_module_path then treats the trailing `// legacy` as part of the path.

**Why it matters**: Produces incorrect ProjectUnit.path entries that will not join to anything in tokei_files, silently dropping LOC/file enrichment for that unit while still listing it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Trim everything from // onward before pushing the directive (both block and single-line branches)
- [ ] #2 Test covers ./api // legacy inside a use block and use ./mymod // note single-line
<!-- AC:END -->
