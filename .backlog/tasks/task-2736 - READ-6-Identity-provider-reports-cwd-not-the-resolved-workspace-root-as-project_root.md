---
id: TASK-2736
title: 'READ-6: Identity provider reports cwd, not the resolved workspace root, as project_root'
status: Done
assignee: []
created_date: '2026-11-18 18:37'
updated_date: '2026-11-18 22:28'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2849'
modified_files:
  - extensions-rust/about/src/identity/mod.rs
priority: medium
ordinal: 1000
dedup_key: 'READ-6:extensions-rust/about/src/identity/mod.rs:RustIdentityProvider::provide'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/identity/mod.rs:45` (`RustIdentityProvider::provide`: `let cwd = ctx.working_directory_arc().clone();`, passed to `resolve_identity_fields` and `build_identity_value(.., &cwd)`)

**What**: `load_workspace_manifest` resolves the workspace root via an ancestor walk, and `LoadedManifest::workspace_root` documents that providers "must join member paths onto *this*, never onto `ctx.working_directory`". The units and coverage providers follow that; the identity provider does not. It hands the live process cwd to `build_identity_value`, which emits it as `project_root`, and to the git repository fallback. Run `ops about` from `crates/foo` and identity reports the workspace's name/version/crate count next to `project_root = .../crates/foo`.

**Why it matters**: The identity card mixes two roots: every manifest-derived field describes the workspace while `project_root` describes wherever the user happened to stand. No test covers identity from a member subdirectory, so the mismatch is unpinned.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 RustIdentityProvider::provide passes manifest.workspace_root() (not ctx.working_directory) to build_identity_value and the repository fallback
- [x] #2 A test runs the identity provider with the working directory set to a member crate and asserts project_root equals the workspace root

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.11 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
