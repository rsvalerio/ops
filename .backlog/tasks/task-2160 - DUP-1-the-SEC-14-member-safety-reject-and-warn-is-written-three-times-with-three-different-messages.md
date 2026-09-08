---
id: TASK-2160
title: >-
  DUP-1: the SEC-14 member-safety reject-and-warn is written three times with
  three different messages
status: Done
assignee:
  - TASK-2236
created_date: '2026-09-08 07:05'
updated_date: '2026-09-08 15:53'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions-rust/about/src/members.rs
  - extensions-rust/about/src/units.rs
priority: low
ordinal: 73000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/members.rs:56-62`, `extensions-rust/about/src/units.rs:100-109`, `extensions-rust/about/src/units.rs:264-271`

**What**: `member_path_is_workspace_safe` is the single predicate, but the "call it, warn, and drop the member" wrapper around it is copied three times, each with its own warn message and its own field spelling:

- `members::resolved_workspace_members` — `member = %member`, "SEC-14 / TASK-1246: workspace member is absolute or contains `..`; dropping"
- `units::member_is_unit_safe` — `member = %member`, "SEC-14 / TASK-1246: rejecting absolute or `..` workspace member in units provider"
- `units::resolve_crate_display_name` — `member = %member`, "SEC-14 / TASK-1246: rejecting absolute or `..` workspace member in display-name resolver"

All three use `%member` (Display) for a manifest-controlled string. That is inconsistent with the rest of the crate, which deliberately Debug-formats manifest-controlled text for exactly this reason: `members.rs:157-166` and `:176-179` (`pattern = ?entry`), `:365-372` (`pattern = ?member`), `units.rs:216-220` and `:232-236` (`path = ?…`), each citing ERR-7 / TASK-0941 / TASK-0977 — "Debug-format the manifest-controlled pattern so embedded newlines / ANSI escapes cannot forge log records". A `[workspace].members` entry is the same attacker-controlled surface as a `[workspace].exclude` pattern; here it reaches the log through `%`.

**Why it matters**: Two costs. (1) The rejection policy has one predicate but three call-site wrappers, so a change to what rejection *means* (adding a symlink or prefix check, downgrading the warn to debug, adding a field) has three places to land and no compiler signal if one is missed. (2) The `%member` spelling is a live log-injection hole in the crate's own stated ERR-7 policy: a member entry containing `\n` or an ANSI escape splits or reformats the log record, and it is precisely the *rejected* (hostile-shaped) entries that reach these three sites.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 One helper in `members.rs` performs the check and emits the warn, taking the call-site name as a structured field; `resolved_workspace_members`, `member_is_unit_safe` and `resolve_crate_display_name` all route through it
- [ ] #2 The member value is Debug-formatted (`member = ?member`) in the shared warn, matching the ERR-7 / TASK-0941 policy the sibling breadcrumbs in the same two files already follow
- [ ] #3 A test drives a rejected member containing an embedded newline and an ANSI escape through the units provider and asserts the captured log stays on one line and contains no raw ESC — mirroring `crate_metadata_breadcrumbs_debug_escape_control_characters` in `units.rs`
- [ ] #4 The defence-in-depth intent is preserved: each of the three call sites still rejects independently rather than relying on an upstream filter
<!-- AC:END -->
