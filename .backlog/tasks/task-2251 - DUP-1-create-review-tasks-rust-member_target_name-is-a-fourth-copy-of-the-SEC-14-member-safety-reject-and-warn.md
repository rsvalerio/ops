---
id: TASK-2251
title: >-
  DUP-1: create-review-tasks-rust member_target_name is a fourth copy of the
  SEC-14 member-safety reject-and-warn
status: Triage
assignee: []
created_date: '2026-09-08 16:02'
updated_date: '2026-09-08 16:07'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions-rust/create-review-tasks/src/provider.rs
priority: low
ordinal: 157000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/create-review-tasks/src/provider.rs:112-120`

**What**: TASK-2160 unified the SEC-14 member-safety reject-and-warn at
three sites into `ops_about_rust::members::member_path_is_workspace_safe_or_warn`
(emit shared breadcrumb with `site` field, Debug-formatted member).
`member_target_name` in create-review-tasks-rust is a fourth check-and-warn
copy. It already Debug-formats (`member = ?member`) and its message is
accurate for its context, so it is not a live ERR-7 hole — but a future
change to the rejection breadcrumb shape now has two places to land instead
of one, which is the duplication TASK-2160 existed to remove.

**Why it matters**: The shared helper is currently `pub(crate)` in
ops-about-rust; promoting it to `pub` (it is already re-exported beside the
public `member_path_is_workspace_safe`) and routing this site through it
with `site = "create-review-tasks provider"` is a mechanical follow-up that
keeps the message context via the structured field.

**Origin**: discovered during TASK-2236 (wave2) while fixing TASK-2160; left
out of the wave because the file is outside wave2's scope and overlapping
waves are in flight.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 member_path_is_workspace_safe_or_warn is pub and member_target_name routes through it with a site field, with no behaviour change beyond the unified breadcrumb
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Update from wave2: clippy (redundant_pub_crate) forced the helper to `pub fn` inside the private `members` module, so the remaining work is only the lib.rs re-export beside `member_path_is_workspace_safe` plus rerouting the call site.
<!-- SECTION:NOTES:END -->
