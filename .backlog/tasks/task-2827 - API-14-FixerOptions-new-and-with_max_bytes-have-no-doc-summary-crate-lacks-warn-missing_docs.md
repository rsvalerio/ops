---
id: TASK-2827
title: 'API-14: FixerOptions::new and with_max_bytes have no doc summary; crate lacks warn(missing_docs)'
status: To Do
assignee: []
created_date: '2026-11-18 18:49'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - API
dependencies: []
parent_task_id: 'TASK-2859'
modified_files:
  - extensions/text-fixers/src/options.rs
  - extensions/text-fixers/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'API-14:extensions/text-fixers/src/options.rs:FixerOptions'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/text-fixers/src/options.rs:34-42` (`FixerOptions::new`, `FixerOptions::with_max_bytes`), `extensions/text-fixers/src/lib.rs` (crate root)

**What**: Two public items re-exported from the crate root (`ops_text_fixers::FixerOptions`) carry no `///` summary: `FixerOptions::new` (which silently picks `DEFAULT_MAX_BYTES` and `check: false` defaults a caller needs to know about) and `FixerOptions::with_max_bytes`. Every other public item in the crate is documented. The sibling `ops-config-checkers` crate enforces this with `#![warn(missing_docs)]`; `ops-text-fixers` does not, so nothing catches the gap.

**Why it matters**: `new`'s defaults (size cap, write mode) are behaviour the CLI depends on; without a doc they are discoverable only by reading the body. A lint keeps the next public item from regressing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 FixerOptions::new and with_max_bytes have doc summaries; new names the defaults it applies (max_bytes = DEFAULT_MAX_BYTES, check = false)
- [ ] #2 lib.rs carries #![warn(missing_docs)] and the crate builds clean under it
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.31 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
