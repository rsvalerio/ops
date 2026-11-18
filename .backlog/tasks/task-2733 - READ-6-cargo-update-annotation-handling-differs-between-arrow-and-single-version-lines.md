---
id: TASK-2733
title: 'READ-6: cargo-update annotation handling differs between arrow and single-version lines'
status: Done
assignee: []
created_date: '2026-11-18 18:37'
updated_date: '2026-11-18 22:41'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2851'
modified_files:
  - extensions-rust/cargo-update/src/lib.rs
  - extensions-rust/cargo-update/src/tests.rs
priority: medium
ordinal: 1000
dedup_key: 'READ-6:extensions-rust/cargo-update/src/lib.rs:is_available_annotation'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
<!-- scan confidence: candidates to inspect -->
**File**: `extensions-rust/cargo-update/src/lib.rs:545` (`is_available_annotation`), `:590` (`parse_arrow_line`), `:617` (`parse_single_version_line`)

**What**: The two line parsers treat cargo's trailing latest-version annotation inconsistently. `parse_arrow_line` (Updating/Downgrading) accepts exactly `(available: vX)` silently via `is_available_annotation`, but `parse_single_version_line` (Adding/Removing) warns on *any* trailing token. Cargo's lockfile printer appends the same latest/MSRV note to `Adding` lines (`Adding foo v0.1.0 (available: v0.2.0)`), so a healthy run emits the "unexpected trailing tokens; annotation discarded" warn. Separately, `is_available_annotation` accepts only the 3-token form, so cargo's MSRV-aware variants — `(available: v1.2.0, requires Rust 1.85)` and `(requires Rust 1.85)` — also warn as format drift on Updating lines.

Candidates to confirm against the cargo version in use (cargo 1.98 observed `Updating jiff v0.2.35 -> v0.2.46 (available: v1.0.1)` locally; the Adding/MSRV forms were not reproduced in this repo):
- `Adding <name> v<ver> (available: v<ver>)`
- `Updating <name> v<a> -> v<b> (available: v<c>, requires Rust <msrv>)`
- `Updating <name> v<a> -> v<b> (requires Rust <msrv>)`

**Why it matters**: The default-level warn is the crate's format-drift signal. Firing it on routine cargo output trains operators to ignore it, so real drift goes unnoticed, and it adds noise to every `ops about --update` refresh.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Adding/Removing lines share the Updating/Downgrading annotation acceptance (one helper used by both parsers)
- [x] #2 Cargo's documented latest/MSRV annotation shapes are accepted without a warn, verified against cargo's lockfile-change printer source
- [x] #3 Tests cover Adding with (available: vX) and the requires-Rust variants, asserting no warn and correct version fields
- [x] #4 Genuinely unknown trailing tokens still warn

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.13 (code-review-rust run, 2026-11-18).

Shapes verified against cargo rust-1.98.0 src/cargo/ops/cargo_update.rs (report_required_rust_version, report_too_new, report_latest): (requires Rust R), (published <age>, minimum age <age>), (available: vX[, requires Rust R][, published <age>]). Removing lines carry no note there but share the helper per AC1.

<!-- SECTION:NOTES:END -->
