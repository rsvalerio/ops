---
id: TASK-2366
title: 'FN-1: parse_action_line is ~125 lines and relies on unreachable! arms for an invariant ACTION_PREFIXES holds only by convention'
status: Done
assignee: []
created_date: '2026-10-04 14:11'
updated_date: '2026-10-04 15:27'
labels:
  - code-review-rust
  - FN
dependencies: []
parent_task_id: 'TASK-2417'
modified_files:
  - extensions-rust/cargo-update/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'FN-1:extensions-rust/cargo-update/src/lib.rs:parse_action_line'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:407-499` (`parse_action_line`; table at `ACTION_PREFIXES` ~line 267)

**What**: `parse_action_line` is about 125 lines and mixes verb dispatch, tokenisation, trailing-token warnings, control-character validation and entry construction in two near-duplicate branches (Arrow vs single-version: the same `is_control_free(name)` and `is_version_shaped` rejection logic and the same name/strip_v_prefix construction appear twice). `ACTION_PREFIXES` pairs an `UpdateAction` with a redundant `VersionShape`, so two `#[allow(clippy::unreachable)] unreachable!` arms exist to cover combinations the types allow but the parser never produces (CL-3: the precondition lives in a table convention, not in a type).

**Why it matters**: The function is the crate's core and the most-edited spot when cargo changes its output. Length and the duplicated validation make drift between the two branches likely, and the unreachable arms would panic on a bad table edit.

**Suggested shape**: derive the shape from the action (a method on `UpdateAction`) or make the table entry a single enum so no impossible pairs exist; extract `validate_name`/`validate_versions` and `parse_arrow_line` / `parse_single_version_line` helpers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 parse_action_line is split into helpers each under 50 lines
- [x] #2 No unreachable! arms remain; Arrow-vs-single-version pairing cannot be expressed incorrectly
- [x] #3 Name and version validation exists in one place

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Landed in wave TASK-2417. VersionShape removed: ACTION_PREFIXES maps verb to UpdateAction only, and parse_action_line matches exhaustively on the action to pick parse_arrow_line or parse_single_version_line plus the entry variant, so no unreachable! arm remains. Name and version validation lives in validate_fields.
<!-- SECTION:NOTES:END -->
