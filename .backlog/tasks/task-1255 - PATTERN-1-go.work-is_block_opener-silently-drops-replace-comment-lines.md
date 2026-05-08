---
id: TASK-1255
title: 'PATTERN-1: go.work is_block_opener silently drops replace(// comment lines'
status: To Do
assignee:
  - TASK-1270
created_date: '2026-05-08 13:01'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - patterns
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/go_work.rs:52`

**What**: `is_block_opener` strips the keyword prefix and runs `strip_line_comment` on the rest. `strip_line_comment` only fires when `//` is at SOL or follows ASCII whitespace. A `replace(// comment` line yields `rest = "(// comment"`, the strip is a no-op, the trimmed value is `"(// comment"` not `"("`, and the function returns false. cmd/go itself accepts this shape — the parser silently ignores the entire block.

**Why it matters**: Sister case of TASK-0994. cmd/go accepts `replace(// note` and `use(// note` — the comment after the bare paren is legal go.mod syntax. The current parser drops every member from the About card module count.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 is_block_opener recognises an inline // comment immediately after ( with no whitespace
- [ ] #2 Regression test for both replace(//note and use(//note populating their respective lists
- [ ] #3 strip_line_comment policy unchanged so embedded // in tokens still survives
<!-- AC:END -->
