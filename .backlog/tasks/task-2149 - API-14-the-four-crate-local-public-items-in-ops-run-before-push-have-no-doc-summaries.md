---
id: TASK-2149
title: >-
  API-14: the four crate-local public items in ops-run-before-push have no doc
  summaries
status: To Do
assignee:
  - TASK-2247
created_date: '2026-09-08 07:03'
updated_date: '2026-09-08 11:00'
labels:
  - code-review-rust
  - api-design
dependencies: []
modified_files:
  - extensions/run-before-push/src/lib.rs
priority: low
ordinal: 62000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-push/src/lib.rs:12`, `:13`, `:14`, `:16`

**What**: Every public item this crate declares by hand is undocumented, while the items it derives from the ref-update logic below are documented carefully:

- `pub const NAME: &str` (:12)
- `pub const DESCRIPTION: &str` (:13)
- `pub const SHORTNAME: &str` (:14)
- `pub struct RunBeforePushExtension` (:16)

`NAME`/`SHORTNAME` are the identifiers the extension registry, the CLI dispatch table and the installed hook script all agree on, and `hook_config_pins_every_macro_argument` exists precisely because getting one of them wrong is a copy-paste hazard between this crate and `ops-run-before-commit` — yet nothing in the rendered docs says which of the three is the user-typed subcommand, which is the registry key, and why `SHORTNAME` must equal `NAME` here when the concept exists to let them differ.

This is the crate-local counterpart of TASK-2124 (same rule, same items, `run-before-commit`) and is disjoint from TASK-2139 (the macro-generated items, owned by `ops-hook-common`).

**Why it matters**: These are the crate's entire hand-written public surface and they are what a second hook extension would be copied from. `#[must_use]`-adjacent constants with no summary give the next author no way to tell which one is safe to change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 NAME, DESCRIPTION, SHORTNAME and RunBeforePushExtension each carry a doc summary stating what the value is used for
- [ ] #2 The NAME / SHORTNAME docs say which one the user types, which one the registry keys on, and why they are equal in this crate
- [ ] #3 cargo doc -p ops-run-before-push shows no undocumented public item in this crate's own module
<!-- AC:END -->
