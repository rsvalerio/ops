---
id: TASK-1203
title: >-
  ERR-1: parse_upgrade_table_inner re-arms columns to None on every
  header-shaped line, dropping body rows
status: Done
assignee:
  - TASK-1267
created_date: '2026-05-08 08:16'
updated_date: '2026-05-09 14:43'
labels:
  - code-review-rust
  - err
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse.rs:147-153`

**What**: The inner parser's header detection sets `columns = None; saw_recognised_header = true; continue;` on any matching line. cargo-edit's main header is the only line that should match in practice, but a future cargo-edit may emit a per-package sub-table header, or a localised note row could happen to contain the literal substrings. Once columns is reset to None, every subsequent body row is dropped until the next ==== separator restores them.

**Why it matters**: The current shape conflates "I have not yet seen a header" with "I just saw another header". A more conservative rule: detect the header once, ignore subsequent header-shaped lines (or raise a warn for the inverse). Also makes saw_recognised_header diagnostic flag unreliable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 After the first header detection, parse_upgrade_table_inner keeps the existing columns until the next ==== row replaces them; a synthetic input with two header-shaped lines and one separator parses every row that follows the separator.
- [x] #2 An explicit test pins the multi-header behaviour and asserts the parsed entry count matches the body row count.
<!-- AC:END -->
