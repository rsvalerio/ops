---
id: TASK-2711
title: 'API-13: ops-theme exposes style helpers under two public paths'
status: Done
assignee: []
created_date: '2026-11-18 18:32'
updated_date: '2026-11-18 22:07'
labels:
  - code-review-rust
  - api-design
dependencies: []
parent_task_id: 'TASK-2845'
modified_files:
  - crates/theme/src/lib.rs
  - crates/theme/src/style.rs
priority: low
ordinal: 1000
dedup_key: 'review-2667.06-4'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/lib.rs:29` (`pub mod style`), `crates/theme/src/lib.rs:39` (`pub use style::{apply_style, strip_ansi, strip_ansi_preserving_raw, visible_width}`)

**What**: `style` is a public module, and its items are re-exported at the crate root too, so `apply_style`, `strip_ansi`, `strip_ansi_preserving_raw` and `visible_width` each have two public paths (`ops_theme::visible_width` and `ops_theme::style::visible_width`). The workspace already uses both: `ops_theme::style::visible_width` (5 sites) next to `ops_theme::strip_ansi_preserving_raw` (root path) and `theme::apply_style` (runner `display/finalize.rs`). The other `style` items (`truncate_to_width`, `precompute_sgr_prefix`, `ELLIPSIS`, ...) are only under `style::`, so the root set is an arbitrary subset. `style.rs` also documents `sgr`'s exports as "rendering crate internal API" while making them `pub`.

**Why it matters**: Two paths for one item doubles it in rustdoc and search, and leaves callers unsure which is canonical, so imports drift (they already have). This is the leftover-compatibility-re-export shape API-13 targets.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each style helper has exactly one public path: either style becomes a private module whose items are lifted to the root, or the root re-exports are removed and callers import from ops_theme::style
- [x] #2 Workspace callers (runner, cargo-update, others found by grep) use the single remaining path and cargo build --workspace passes

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.06 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
