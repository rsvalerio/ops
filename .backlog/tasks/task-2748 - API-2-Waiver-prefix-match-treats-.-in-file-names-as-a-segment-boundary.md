---
id: TASK-2748
title: 'API-2: Waiver prefix match treats ''.'' in file names as a segment boundary'
status: To Do
assignee: []
created_date: '2026-11-18 18:40'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2852'
modified_files:
  - extensions-rust/foundation/src/lib.rs
  - extensions-rust/foundation/src/tests.rs
priority: medium
ordinal: 1000
dedup_key: 'API-2:extensions-rust/foundation/src/lib.rs:waiver_covers'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/foundation/src/lib.rs:563` (`waiver_covers`), used by `apply_waivers` (`lib.rs:569`)

**What**: Drift locations are plain strings (`<file>` or `<file>:<dotted.key>`), and `waiver_covers` accepts a waiver when the location starts with it and the rest begins with `:` or `.`. File names contain dots, so the `.` boundary also fires inside the file name:

- waiver `Cargo` covers `Cargo.toml:workspace.lints.clippy.panic` (rest = `.toml:...`), so it waives every lint-policy drift in the root manifest
- waiver `deny` covers `deny.toml`, `clippy` covers `clippy.toml`, `.config/nextest` covers `.config/nextest.toml`
- an empty waiver key `"" = "..."` covers every location that starts with `.`, such as `.ops.toml:*` and `.config/nextest.toml`
- `.ops.toml:cargo.workspaces.<dir>` puts a directory name in a dotted key, so a waiver for nested workspace `tools` also covers `tools.x`

The existing test `waiver_prefix_matching_respects_segment_boundaries` only checks boundaries inside the key path, not inside the file name.

**Why it matters**: The check exists to surface drift unless a waiver deliberately records why. A short or mistyped waiver key silently suppresses a whole file's or the whole lint table's drift, and is still counted as used, so it never shows up in `unused_waivers`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A waiver matches only when its file part equals the location's file exactly, and its key part is a whole-segment prefix of the location's key path
- [ ] #2 Waivers `Cargo`, `deny` and the empty string cover nothing, and tests assert this
- [ ] #3 A nested workspace directory name containing '.' cannot be covered by a waiver for a shorter directory name, and a test covers it
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.16 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
