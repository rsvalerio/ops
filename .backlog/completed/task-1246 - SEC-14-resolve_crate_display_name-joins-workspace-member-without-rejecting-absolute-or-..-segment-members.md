---
id: TASK-1246
title: >-
  SEC-14: resolve_crate_display_name joins workspace member without rejecting
  absolute or ..-segment members
status: Done
assignee:
  - TASK-1260
created_date: '2026-05-08 13:00'
updated_date: '2026-05-08 14:20'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/units.rs:70`

**What**: `units.rs::provide` and `resolve_crate_display_name` build `cwd.join(member).join("Cargo.toml")` for every `[workspace].members` entry. `Path::join` discards cwd when member is absolute and walks parents on .. segments, so a hostile root Cargo.toml drives `read_capped_to_string` and tracing breadcrumbs at any filesystem location.

**Why it matters**: Clone-and-preview threat model (CI checkout, IDE preview) lets attacker-controlled member paths reach the per-crate manifest read, leaking presence/parse-error of arbitrary Cargo.toml outside the workspace root. Sister checks (resolved_workspace_members) already filter glob expansions but verbatim members entries skip the scrub.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Reject member entries that are absolute or contain .. before join with a one-shot tracing::warn
- [x] #2 Apply the same scrub at resolved_workspace_members's passthrough arm so non-glob entries cannot bypass it
- [x] #3 Unit test pinning members = ["../escape"] and ["/abs"] produce empty units with a warn
<!-- AC:END -->
