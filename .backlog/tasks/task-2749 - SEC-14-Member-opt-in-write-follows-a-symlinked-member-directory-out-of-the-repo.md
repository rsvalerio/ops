---
id: TASK-2749
title: 'SEC-14: Member opt-in write follows a symlinked member directory out of the repo'
status: Done
assignee: []
created_date: '2026-11-18 18:40'
updated_date: '2026-11-18 22:45'
labels:
  - code-review-rust
  - security
dependencies: []
parent_task_id: 'TASK-2852'
modified_files:
  - extensions-rust/foundation/src/lib.rs
  - extensions-rust/foundation/src/tests.rs
priority: medium
ordinal: 1000
dedup_key: 'SEC-14:extensions-rust/foundation/src/lib.rs:scaffold_member_opt_in'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/foundation/src/lib.rs:291` (`scaffold_member_opt_in`), members from `workspace_members` (`lib.rs:156`)

**What**: `scaffold` appends `[lints] workspace = true` to every member manifest that has no `[lints]`, writing `root.join("<member>/Cargo.toml")` via `atomic_write`. Members are filtered only lexically by `member_path_is_workspace_safe_or_warn` (absolute or `..` rejected). A member directory that is a symlink, e.g. `crates/x -> /home/user/other-project`, passes that check, and `atomic_write` creates its temp file and renames it inside the symlink target, so ops rewrites a `Cargo.toml` outside the repository.

The same module already refuses to write through a symlinked `.config` directory (`ensure_real_dir`, `lib.rs:260`), so the guard is applied to the foundation files but not to member manifests.

**Why it matters**: `ops init --rust` run on a cloned, untrusted repo can modify a manifest anywhere the user can write. The edit is small, but it is a write outside the tree the command claims to scaffold.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 scaffold refuses (error or skip with a reported outcome) to write a member manifest whose member directory or manifest path resolves outside the repo root, e.g. through a symlink
- [x] #2 A test with a member directory symlinked outside the temp workspace shows the outside Cargo.toml is left unchanged

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.16 (code-review-rust run, 2026-11-18).

scaffold canonicalizes the root and every member manifest before any write and refuses (error) one resolving outside the repo; in-repo symlinked members still work. Tests: a_member_symlinked_out_of_the_repo_is_refused (outside Cargo.toml unchanged, nothing written), a_member_symlinked_inside_the_repo_is_written.

<!-- SECTION:NOTES:END -->
