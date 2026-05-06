---
id: TASK-0960
title: 'ERR-1: separator_columns byte-index slicing panics on non-ASCII row'
status: To Do
assignee:
  - TASK-1013
created_date: '2026-05-04 21:46'
updated_date: '2026-05-06 06:48'
labels:
  - code-review-rust
  - correctness
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse.rs:194-218, 123-189`

**What**: `separator_columns` finds byte offsets of `=` runs in the separator row, and `parse_upgrade_row` uses them via `&line[start..end.min(line.len())]` on data rows. Byte offsets line up with the separator row, but a data row containing a multi-byte UTF-8 character (localized note text, Unicode crate name) can land start/end mid-codepoint and panic.

**Why it matters**: cargo-edit output is en_US today but a future localization or a manifest with non-ASCII metadata in `note` could panic the entire `ops deps` flow.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Slice via line.is_char_boundary clamp or char_indices; fall back to None+warn rather than panic when boundary checks fail
- [ ] #2 Test row containing a non-ASCII character that crosses a column edge does not panic
<!-- AC:END -->
