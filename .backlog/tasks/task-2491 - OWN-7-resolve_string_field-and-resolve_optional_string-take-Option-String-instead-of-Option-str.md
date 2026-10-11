---
id: TASK-2491
title: 'OWN-7: resolve_string_field and resolve_optional_string take Option<&String> instead of Option<&str>'
status: Done
assignee: []
created_date: '2026-10-10 15:30'
updated_date: '2026-10-10 21:41'
labels:
  - code-review
  - own
dependencies: []
parent_task_id: 'TASK-2615'
modified_files:
  - extensions-rust/cargo-toml/src/inheritance.rs
priority: low
ordinal: 1000
dedup_key: 'OWN-7:extensions-rust/cargo-toml/src/inheritance.rs:resolve_string_field'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/inheritance.rs:105`

**What**: Two function signatures borrow the owned type instead of the borrowed one:
- inheritance.rs:105 — `pub fn resolve_string_field(field: &mut InheritableString, ws_value: Option<&String>)`
- inheritance.rs:130 — `pub fn resolve_optional_string(field: &mut Option<InheritableString>, ws_value: Option<&String>)`

Callers pass `ws_pkg.version.as_ref()` (inheritance.rs:72-79, 91) to satisfy the `&String` shape. `Option<&str>` via `.as_deref()` is the borrowed form: `&String` is two indirections to the bytes and rejects callers that only hold a `&str` (a `&str` cannot coerce back up to `&String`). `clippy::ptr_arg` does not fire on the `Option<&String>` shape, which is why this survived the workspace lint gates.

**Why it matters**: OWN-7 — where a parameter is a plain reference, borrow the borrowed type (`&str`, not `&String`). The functions are crate-visible (pub in a private module), so the fix is mechanical: change the parameter to `Option<&str>` and the call sites to `.as_deref()`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 resolve_string_field and resolve_optional_string accept Option<&str> (or &str where Option is not needed)
- [x] #2 Call sites in resolve_package_inheritance updated (as_deref or equivalent) and cargo check/clippy pass

<!-- AC:END -->
