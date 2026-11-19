---
id: TASK-2734
title: 'SEC-14: Literal workspace members skip the symlink containment check glob members get'
status: To Do
assignee: []
created_date: '2026-11-18 18:37'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2849'
modified_files:
  - extensions-rust/about/src/members.rs
  - extensions-rust/about/src/units.rs
  - extensions-rust/about/src/manifest.rs
priority: medium
ordinal: 1000
dedup_key: 'SEC-14:extensions-rust/about/src/members.rs:resolved_workspace_members'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/members.rs:70` (`resolved_workspace_members`, `MemberShape::Literal` arm), consumed at `extensions-rust/about/src/units.rs:192` (`build_unit`), `extensions-rust/about/src/units.rs:310` (`resolve_crate_display_name`) and `extensions-rust/about/src/manifest.rs:133` (`LoadedManifest::canonical_member_manifests`)

**What**: Glob-expanded members go through `resolve_glob_entry`, which canonicalizes each entry and rejects it unless it `starts_with(canonical_root)` ("a member reached through [a symlink] must still resolve inside `canonical_root`"). Literal members only pass `member_path_is_workspace_safe` (no absolute path, no `..`) and are then `workspace_root.join(member).join("Cargo.toml")`-ed and read by `read_crate_metadata`. A literal member such as `crates/foo` where `crates/foo` is a symlink to a directory outside the checkout is followed without any containment check, and `canonical_member_manifests` happily canonicalizes it to the out-of-tree path.

**Why it matters**: A hostile repository can make `ops about` read a `Cargo.toml` outside the workspace and surface its name/version/description, and breadcrumbs name out-of-tree paths. The defence exists for globs but not for the equivalent literal spelling, so the policy is inconsistent and the comment in `resolve_glob_entry` overstates the guarantee.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Literal members are canonicalized and rejected (with the shared Debug-formatted warn) when they resolve outside the canonical workspace root, matching resolve_glob_entry
- [ ] #2 A test plants a literal member that is a symlink to a directory outside the workspace and asserts it is dropped from resolved_workspace_members and never read
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.11 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
