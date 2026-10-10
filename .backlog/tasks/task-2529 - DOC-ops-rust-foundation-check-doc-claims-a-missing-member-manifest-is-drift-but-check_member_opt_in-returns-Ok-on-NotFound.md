---
id: TASK-2529
title: 'DOC: ops-rust-foundation check() doc claims a missing member manifest is drift, but check_member_opt_in returns Ok on NotFound'
status: Done
assignee: []
created_date: '2026-10-10 15:34'
updated_date: '2026-10-10 21:42'
labels:
  - code-review
  - documentation
dependencies: []
parent_task_id: 'TASK-2616'
modified_files:
  - extensions-rust/foundation/src/lib.rs
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/foundation/src/lib.rs:412-414` and `:530`

**What**: the `check` doc says a *missing* member manifest "is drift, not an error", but `check_member_opt_in` silently returns clean `Ok(())` on NotFound. The path is reachable: literal `workspace.members` entries are pushed by `resolved_workspace_members` (extensions-rust/about/src/members.rs:72) with no manifest-existence check — only glob expansion filters for Cargo.toml.

**Why it matters**: doc or code is wrong; operators reading the doc will expect drift output that never comes. Surfaced by the TASK-2351.16 review (no rule ID stretched to cover it).

**Source**: TASK-2351.16 review of ops-rust-foundation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Either check_member_opt_in reports NotFound as drift, or the check() doc is corrected to match the skip behaviour

<!-- AC:END -->
