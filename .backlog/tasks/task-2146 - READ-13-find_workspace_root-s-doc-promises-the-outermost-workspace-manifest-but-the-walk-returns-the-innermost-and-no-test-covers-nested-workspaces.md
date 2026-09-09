---
id: TASK-2146
title: 'READ-13: find_workspace_root''s doc promises the outermost workspace manifest but the walk returns the innermost, and no test covers nested workspaces'
status: To Do
assignee: []
created_date: '2026-09-08 07:02'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2248'
modified_files:
  - extensions-rust/cargo-toml/src/workspace_root.rs
  - extensions-rust/cargo-toml/src/lib.rs
  - extensions-rust/cargo-toml/src/tests/find_root.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/workspace_root.rs:54`

**What**: The doc comment states:

> TASK-0501: prefers the *outermost* `Cargo.toml` containing `[workspace]` over the first `Cargo.toml` encountered.

The implementation does not do that. `walk_ancestors` climbs from the start directory upward and returns on the **first** `CandidateAction::AcceptWorkspace` (`workspace_root.rs:387`), which the lenient check emits for the first ancestor whose manifest declares `[workspace]` (`workspace_root.rs:127`). Climbing upward, the first such ancestor is the **innermost** workspace, not the outermost. The claim is only accurate against the narrower thing TASK-0501 actually fixed — preferring a workspace manifest over a nearer *member* manifest.

The distinction is observable for nested workspaces (a workspace whose member is itself a workspace root, e.g. a vendored or example sub-workspace): the walk returns the inner one; the doc says the outer one. `src/tests/find_root.rs` has no nested-workspace test — `find_root_prefers_workspace_over_member` and `find_root_falls_back_to_nearest_when_no_workspace_in_chain` both build chains with a single `[workspace]` — so neither behavior is pinned.

Secondary, same class (a doc claim the code contradicts), `extensions-rust/cargo-toml/src/lib.rs:79-84`: the re-export rationale says without re-exporting `InheritableField` a consumer "could not write the type in a signature, match on `Value` vs `Inherited`, or **construct a `Package`**". `Package` is `#[non_exhaustive]` (`types.rs:111`), so downstream crates cannot construct it with or without those re-exports; the first two reasons stand, the third does not.

**Why it matters**: `find_workspace_root` is the crate's most-read public function and its doc is the contract three extensions resolve their roots against; a reader planning nested-workspace behavior from this paragraph gets the opposite of what runs. The absent test means either reading could be changed silently.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 find_workspace_root's doc states the actual rule: the nearest ancestor manifest declaring [workspace] wins, with the first-seen manifest as fallback when none does
- [ ] #2 A test in src/tests/find_root.rs builds a nested workspace (inner and outer manifests both declaring [workspace]) and pins which one is returned
- [ ] #3 The lib.rs re-export rationale no longer claims downstream code can construct a #[non_exhaustive] Package
<!-- AC:END -->
