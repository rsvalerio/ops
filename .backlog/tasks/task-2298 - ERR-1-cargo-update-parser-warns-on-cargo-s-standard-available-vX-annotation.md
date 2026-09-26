---
id: TASK-2298
title: 'ERR-1: cargo-update parser warns on cargo''s standard ''(available: vX)'' annotation'
status: To Do
assignee: []
created_date: '2026-09-26 19:08'
updated_date: '2026-09-26 19:55'
labels:
  - code-review-rust
  - ERR
dependencies: []
parent_task_id: 'TASK-2304'
modified_files:
  - extensions-rust/cargo-update/src/lib.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs`

**What**: `parse_update_output` emits a `tracing::warn!` ("`Updating`/`Downgrading` line has unexpected trailing tokens; annotation discarded") for lines like `Updating comfy-table v7.1.4 -> v7.2.2 (available: v8.0.1)`. That trailing `(available: …)` annotation is standard modern cargo output whenever a newer semver-incompatible release exists, so the warning fires on healthy runs and leaks onto the user's terminal (seen on `ops about dependencies --duplicates`).

**Why it matters**: a warning on every normal run is noise that trains users to ignore real warnings; the entry itself parses fine.

**Origin**: discovered during TASK-2292 while fixing TASK-2288.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The '(available: vX)' annotation is recognised and parsed (or silently accepted) without a warn
- [ ] #2 A test pins that a line carrying it produces the entry and no warn
<!-- AC:END -->
