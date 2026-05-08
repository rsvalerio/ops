---
id: TASK-1216
title: >-
  PATTERN-1: parse_use_dirs in_use_block branch silently swallows nested use(
  openers
status: To Do
assignee:
  - TASK-1270
created_date: '2026-05-08 08:20'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/go_work.rs:14-39`

**What**: Inside parse_use_dirs, once in_use_block is true, every non-) non-comment line is treated as a directory entry. cmd/go itself rejects nested `use (` openers, but the parser never re-checks is_block_opener while inside a block — a malformed go.work with a stray `use(` line inside an outer block is treated as a directory whose name is `use(`, which then flows into cwd.join('use(') and on into tracing::warn! and About-card output.

**Why it matters**: Operator-controlled config so impact is bounded, but the parser already documents itself as cmd/go-shaped (TASK-0994 / TASK-0976) and silently accepting `use(` as a directory name violates the documented mental model.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 When in_use_block is true, the parser either rejects (with tracing::warn!) any line that matches is_block_opener('use', line), or terminates the outer block before re-entering — pin the chosen behaviour in the rustdoc.
- [ ] #2 A new test parse_use_dirs_warns_on_nested_block_opener writes a go.work containing a nested use ( and asserts the resulting dirs does not contain 'use ('.
<!-- AC:END -->
