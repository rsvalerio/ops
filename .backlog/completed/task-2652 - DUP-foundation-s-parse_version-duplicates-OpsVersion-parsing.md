---
id: TASK-2652
title: 'DUP: foundation''s parse_version duplicates OpsVersion parsing'
status: Done
assignee: []
created_date: '2026-11-01 00:41'
updated_date: '2026-11-01 13:45'
labels:
  - code-review-rust
  - DUP
dependencies: []
parent_task_id: 'TASK-2655'
modified_files:
  - extensions-rust/foundation/src/lib.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/foundation/src/lib.rs:1` (`parse_version`)

**What**: `parse_version` parses `x.y.z` into `[u64; 3]`, and `ops_core::config::OpsVersion` now parses and orders the same text. They differ on a leading `v`, which `OpsVersion` accepts.

**Why it matters**: A pin such as `v0.85.0` is drift to the floor check while the config check reads it as 0.85.0, so the two checks in one `ops init --rust --check` run disagree on the same pin.

**Origin**: discovered during TASK-2646 while fixing TASK-2638.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The foundation floor check and the config check parse a pin the same way

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
foundation's private parse_version is deleted; check_ops_pin parses both the found pin and the template floor through ops_core::config::OpsVersion, the same parser mise_ops_pin uses, so the two checks in one ops init --rust --check run now agree on every pin text, including a leading v (v1.0.0 added to the floor test; previously drift). Ripple: tests.rs gained the v-prefix pin case.
<!-- SECTION:NOTES:END -->
