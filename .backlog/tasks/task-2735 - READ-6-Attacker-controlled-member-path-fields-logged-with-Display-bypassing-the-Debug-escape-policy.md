---
id: TASK-2735
title: 'READ-6: Attacker-controlled member/path fields logged with Display, bypassing the Debug-escape policy'
status: Done
assignee: []
created_date: '2026-11-18 18:37'
updated_date: '2026-11-18 22:27'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2849'
modified_files:
  - extensions-rust/about/src/members.rs
  - extensions-rust/about/src/units.rs
priority: medium
ordinal: 1000
dedup_key: 'READ-6:extensions-rust/about/src/members.rs:resolved_workspace_members'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/members.rs:75` (`resolved_workspace_members`, `MemberShape::Unsupported` warn: `pattern = %member`), `extensions-rust/about/src/units.rs:237` (`resolve_dep_count`: `manifest_path = %key`)

**What**: The crate's stated policy (ERR-7 comments in `members.rs`, `units.rs`, and the tests `glob_prefix_warn_debug_escapes_control_characters` / `crate_metadata_breadcrumbs_debug_escape_control_characters`) is that manifest-controlled member strings and paths derived from them are Debug-formatted (`?`) in tracing fields so embedded newlines / ANSI escapes cannot forge log records. Two sites still use Display (`%`): the unsupported-member-glob warn prints the raw `[workspace].members` entry, and the dep-count debug breadcrumb prints a canonical path that embeds the member directory name.

**Why it matters**: An unsupported-glob entry is by construction attacker-shaped (`crates/{a,b}\n...` reaches exactly this arm after passing the `..`/absolute check), so a hostile `Cargo.toml` can inject a fake log line or terminal escape at warn level. The two exceptions also make the policy unenforceable by reading.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 members.rs unsupported-glob warn and units.rs resolve_dep_count breadcrumb Debug-format the member/path fields
- [x] #2 A capture_tracing test feeds an unsupported member glob containing a newline and ESC and asserts the raw control characters never reach the log line

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.11 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
