---
id: TASK-2676
title: 'DUP-1: bounded_read::safe_line_text re-implements ui::sanitise_line with a different escape format'
status: To Do
assignee: []
created_date: '2026-11-18 18:26'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - DUP
dependencies: []
parent_task_id: 'TASK-2841'
modified_files:
  - crates/core/src/bounded_read.rs
  - crates/core/src/ui.rs
priority: low
ordinal: 1000
dedup_key: 'DUP-1:crates/core/src/bounded_read.rs:safe_line_text'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/bounded_read.rs:244-258` (`safe_line_text`), `crates/core/src/ui.rs:75-96` (`sanitise_line` / `sanitise_to_string`)

**What**: `safe_line_text` walks the string, tests `crate::text::is_unsafe_display_char`, and escapes rejected chars via `format!("{c:?}").trim_matches('\'')` (rendering `\n`, `\u{1b}`). `ui::sanitise_line` is the crate's documented escaping channel for the same policy (same predicate) but renders `\x0a` / `\x1b` / `\u{202e}`. Two escapers for one policy in one crate, with diverging output, plus `safe_line_text` allocates a `String` per character (`format!` + `to_string`) where `sanitise_line` writes into one buffer.

**Why it matters**: The module docs of `bounded_read` stress that hardening is defined once "so a hardening fix applied once reaches every file-walking extension"; a second escaper is the place a future policy change (tab handling, a new escape class) lands in one channel and not the other.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 bounded_read report lines are escaped through ui::sanitise_line / sanitise_to_string (or one shared helper both call)
- [ ] #2 safe_line_text is removed or reduced to a thin call into the shared escaper
- [ ] #3 Existing bounded_read escaping tests are updated to the shared escape format and still pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.03 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
