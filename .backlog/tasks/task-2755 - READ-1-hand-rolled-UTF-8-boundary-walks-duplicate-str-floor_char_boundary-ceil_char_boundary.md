---
id: TASK-2755
title: 'READ-1: hand-rolled UTF-8 boundary walks duplicate str::floor_char_boundary / ceil_char_boundary'
status: To Do
assignee: []
created_date: '2026-11-18 18:41'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2851'
modified_files:
  - extensions-rust/deps/src/parse/mod.rs
  - extensions-rust/deps/src/parse/upgrade.rs
priority: low
ordinal: 1000
dedup_key: 'READ-1:extensions-rust/deps/src/parse/mod.rs:truncate_for_log'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse/mod.rs:26`, `extensions-rust/deps/src/parse/upgrade.rs:361`

**What**: `truncate_for_log` (parse/mod.rs:26-46) uses a `loop { if let Some(head) = s.get(..end) { break head; } end = end.saturating_sub(1); }` to walk back to a char boundary, with two comments explaining why it terminates. `clamp_to_char_boundaries` (parse/upgrade.rs:361-393) has two `while` loops that walk `s` forward and `e` backward with `is_char_boundary` and `saturating_add`/`saturating_sub`, again with comments arguing the bounds. The workspace MSRV is 1.97 (`rust-version` in the root Cargo.toml), and `str::floor_char_boundary` / `str::ceil_char_boundary` have been stable since 1.91. Checked on the installed rustc 1.98: `"héllo".floor_char_boundary(2) == 1` and `ceil_char_boundary(2) == 3`. Each walk is one std call: `&s[..s.floor_char_boundary(MAX)]`, and `ceil_char_boundary(start)` / `floor_char_boundary(clamped_end)`.

**Why it matters**: The loops and the comments that justify them are code a reader has to check by hand. The std calls carry the same guarantee and need no termination argument.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 truncate_for_log uses str::floor_char_boundary instead of the manual walk-back loop
- [ ] #2 clamp_to_char_boundaries uses ceil_char_boundary / floor_char_boundary and keeps its clamp-warning behaviour
- [ ] #3 The existing multi-byte truncation and column-clamp tests pass unchanged
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.15 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
