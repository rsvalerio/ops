---
id: TASK-1115
title: >-
  PATTERN-1: pad_header silently produces an unaligned, separator-free string
  when target_content_width overflows
status: Done
assignee: []
created_date: '2026-05-07 21:51'
updated_date: '2026-05-07 23:19'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/text_util.rs:126`

**What**: `pad_header` computes `padding = target_content_width.saturating_sub(left_display + right_display + 1)` and returns `format!("{}{}{} ", left, " ".repeat(padding), right)`. When `left_display + right_display + 1 > target_content_width` the saturating subtract pins padding at 0 and the function returns `"<left><right> "` — a string with no whitespace separator between the two halves. The doc comment promises "right-aligned right string, one trailing space"; the overflow case silently violates the contract.

**Why it matters**: The about card header is the most visible piece of `ops about` output. On overflow the operator sees two adjacent identifiers concatenated (`Foo bar=BarValue` → `FooBarValue`), which is indistinguishable from an upstream string-concatenation bug and obscures whether the card is rendering correctly. Either return a Result/Option that the caller can branch on, log a debug breadcrumb naming the overflow, or guarantee at least a single space separator regardless of width.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 pad_header overflow case either preserves a minimum 1-char separator, or surfaces overflow via Result/log breadcrumb so misalignment is observable
<!-- AC:END -->
