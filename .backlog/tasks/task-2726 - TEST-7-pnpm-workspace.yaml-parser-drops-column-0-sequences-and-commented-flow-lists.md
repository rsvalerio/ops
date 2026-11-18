---
id: TASK-2726
title: 'TEST-7: pnpm-workspace.yaml parser drops column-0 sequences and commented flow lists'
status: To Do
assignee: []
created_date: '2026-11-18 18:34'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - test
dependencies: []
parent_task_id: 'TASK-2848'
modified_files:
  - extensions-node/about/src/units.rs
priority: medium
ordinal: 1000
dedup_key: 'TEST-7:extensions-node/about/src/units.rs:parse_pnpm_workspace_yaml'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/units.rs:223` (`parse_pnpm_workspace_yaml`, block-end check at line 242, flow-list branch at line 258)

**What**: Two valid YAML shapes for the `packages:` key resolve to zero workspace members, and the only signal is a `tracing::debug!`:

1. A block sequence written at the same indentation as its key, which YAML allows and many formatters emit:
   ```yaml
   packages:
   - 'apps/*'
   - 'libs/*'
   ```
   `packages_indent` is 0, so the first `- 'apps/*'` line (`leading_ws == 0`) meets `leading_ws <= packages_indent` and ends the block before any item is read.
2. A flow list followed by a trailing comment: `packages: [apps/*, libs/*] # members`. `rest.strip_suffix(']')` fails because of the comment, so the line falls into block mode and the next top-level key closes it.

The test module covers indented block lists, flow lists, sibling keys and comments inside items, but neither of these shapes. No rule covers a parser that is simply wrong, so this is filed under TEST-7: the function exceeds the complexity threshold, and these branches have no test.

**Why it matters**: A pnpm monorepo with either layout shows no units in `ops about`, and the identity card's `packages` row is hidden, with no warning at the default log level. The units list and package count are wrong without any visible error.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A `packages:` block whose `- item` lines sit at column 0 yields every item
- [ ] #2 `packages: [a, b] # comment` yields a and b
- [ ] #3 Unit tests pin both shapes, and a sibling top-level key after a column-0 sequence still ends the block
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.09 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
