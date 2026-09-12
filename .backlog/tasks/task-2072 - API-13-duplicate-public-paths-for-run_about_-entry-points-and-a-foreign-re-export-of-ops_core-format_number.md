---
id: TASK-2072
title: 'API-13: duplicate public paths for run_about_* entry points and a foreign re-export of ops_core format_number'
status: Done
assignee: []
created_date: '2026-09-07 22:56'
updated_date: '2026-09-09 18:57'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2247'
modified_files:
  - extensions/about/src/lib.rs
  - extensions/about/src/text_util.rs
priority: low
ordinal: 3000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/lib.rs:30-32,37,42`; `extensions/about/src/text_util.rs:6`

**What**: `lib.rs` declares `pub mod cards; pub mod coverage; pub mod deps; pub mod units;` (plus cfg-gated `code`, `loc`) and *also* re-exports their entry points at the crate root — `pub use coverage::run_about_coverage;`, `pub use deps::run_about_deps;`, `pub use units::run_about_units;`, `pub use code::run_about_code;`, `pub use loc::run_about_loc;`. Every one of those items is therefore reachable by two public paths (`ops_about::run_about_units` and `ops_about::units::run_about_units`), doubling docs/search hits and leaving readers unsure whether the two are the same item.

Additionally `text_util.rs:6` does `pub use ops_core::text::format_number;`, creating a second public alias (`ops_about::text_util::format_number`) for a foreign crate's item. Foreign items should come from their own crate path; the re-export is only consumed internally by `cards.rs`, so a private `use` would do.

**Why it matters**: API-13 — a public item should be reachable by exactly one path; a `pub use` that merely duplicates a still-public module path is the characteristic artifact, and re-exporting another crate's item creates an alias that pins this crate's surface to ops-core's.

Suggested fix (either direction, not both): drop the root `pub use`s and keep the module paths, or make the modules `pub(crate)` and keep the root re-exports as the single public path (the latter matches the crate-root-layout guidance and the convenience role of the root). Demote the `format_number` re-export to a crate-private `use`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each run_about_* entry point is reachable by exactly one public path
- [x] #2 format_number is no longer publicly re-exported from ops_about (import it privately where used)
- [x] #3 No downstream call sites break (grep workspace consumers of ops_about:: before landing)

<!-- AC:END -->
