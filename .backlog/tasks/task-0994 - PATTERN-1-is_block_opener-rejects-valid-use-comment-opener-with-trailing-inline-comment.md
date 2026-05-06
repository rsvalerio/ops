---
id: TASK-0994
title: >-
  PATTERN-1: is_block_opener rejects valid 'use ( // comment' opener with
  trailing inline comment
status: To Do
assignee:
  - TASK-1014
created_date: '2026-05-04 21:59'
updated_date: '2026-05-06 06:48'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/go_work.rs:52-58`

**What**: `is_block_opener(line, "use")` requires the entire trimmed remainder after the keyword to be exactly `"("` — so `use (` and `use(` both succeed, but `use ( // legacy block` fails (the trailing comment is part of `rest` after the strip-prefix and trim_start). cmd/go accepts a trailing line comment on the block opener itself; the parser silently treats the entire `use` block as if the opener were absent, so all subsequent indented entries are dropped.

**Why it matters**: Silent under-counting of `go.work` use directives: an authored line like `use ( // ws-members` causes the entire workspace to be reported as a single-mod project. The same check is reused inside `go_mod.rs::parse` for the `replace ( ... )` block opener (line 44), so the bug applies symmetrically to local-replace counting. The strip-line-comment helper already exists in `go_mod.rs::strip_line_comment` — `is_block_opener` should call it on `rest` before the trim/equality check.

**Candidate fix**:
```rust
pub(crate) fn is_block_opener(line: &str, keyword: &str) -> bool {
    let Some(rest) = line.strip_prefix(keyword) else { return false; };
    let rest = crate::go_mod::strip_line_comment(rest).trim();
    rest == "("
}
```
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 is_block_opener strips a trailing // comment before the equality check
- [ ] #2 Tests cover use ( // comment and replace ( // comment both being recognised as block openers
<!-- AC:END -->
