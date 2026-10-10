---
id: TASK-2425
title: 'ERR-1: foundation check passes clean when workspace members cannot be resolved'
status: Done
assignee: []
created_date: '2026-10-04 15:07'
updated_date: '2026-10-10 14:47'
labels:
  - code-review-rust
  - ERR
dependencies: []
parent_task_id: 'TASK-2438'
modified_files:
  - extensions-rust/foundation/src/lib.rs
  - extensions-rust/foundation/src/tests.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/foundation/src/lib.rs` (`workspace_members`)

**What**: When `ops_cargo_toml::CargoToml::parse` rejects a root manifest that still parses as a TOML table (for example `[workspace] members = "crates/*"`, a string instead of a list), `workspace_members` logs a `could not resolve workspace members` warning and returns an empty list. `check` then skips every member opt-in check and can report no drift, and `scaffold` adds no member opt-ins, while both return success.

**Why it matters**: `ops init --rust --check` gates CI. A check that silently covers fewer members than the workspace has reports a clean result it did not establish; the only signal is a warn-level log line. The behaviour is pinned by `unresolvable_workspace_members_warn_and_skip_the_member_checks` in `extensions-rust/foundation/src/tests.rs`, which will need updating with the fix.

**Origin**: discovered during TASK-2419 while fixing TASK-2380.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A root manifest whose workspace members cannot be resolved makes check report a drift (or return an error) naming Cargo.toml and the parse reason, instead of passing clean
- [x] #2 scaffold returns an error for the same manifest instead of silently adding no member opt-ins

<!-- AC:END -->
