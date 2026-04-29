---
id: TASK-0615
title: 'DUP-3: strip_line_comment duplicated across go_mod.rs and go_work.rs'
status: Done
assignee:
  - TASK-0641
created_date: '2026-04-29 05:20'
updated_date: '2026-04-29 12:07'
labels:
  - code-review-rust
  - DUP
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/go_mod.rs:63`

**What**: The same strip_line_comment helper (3 lines, identical implementation using line.find("//") + slice) appears in both extensions-go/about/src/go_mod.rs:63-68 and extensions-go/about/src/go_work.rs:56-61. Both parsers live in the same crate and a shared `comment` helper would deduplicate.

**Why it matters**: Trivial duplication is cheap to remove and avoids divergence later.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Single strip_line_comment lives in one shared module/file inside extensions-go/about/src
- [ ] #2 Both go_mod.rs and go_work.rs consume that helper
- [ ] #3 Existing tests pass with no behaviour change
<!-- AC:END -->
