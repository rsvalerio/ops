---
id: TASK-2171
title: >-
  PATTERN-1: a root package that is also the workspace root is silently dropped
  from the Rust review targets
status: To Do
assignee:
  - TASK-2243
created_date: '2026-09-08 07:10'
updated_date: '2026-09-08 10:57'
labels:
  - code-review-rust
  - patterns
dependencies: []
modified_files:
  - extensions-rust/create-review-tasks/src/provider.rs
priority: high
ordinal: 84000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/create-review-tasks/src/provider.rs:44-56`

**What**: `provide` treats "has resolvable `[workspace].members`" and "is a single package" as mutually exclusive:

```rust
let members = resolved_workspace_members(&manifest, &root);
let mut targets = if members.is_empty() {
    vec![(root_package_name(&manifest, &root)?, ROOT_PACKAGE_PATH.to_string())]
} else {
    members.iter().map(...).collect()
};
```

`ops_about_rust::resolved_workspace_members` (extensions-rust/about/src/members.rs:46) reads *only* `[workspace].members`; Cargo never requires the root package to be listed there because a root `[package]` is an implicit member of its own workspace. So for the very common hybrid manifest — a `Cargo.toml` carrying both `[package]` and `[workspace] members = [...]` (a library plus an `xtask`/`fuzz`/`examples` member, for instance) — `members` is non-empty, the `else` branch runs, and the root package never becomes a review target. `root_package_name` is reachable only when the member list is empty.

The failure is silent: the payload is well-formed, the engine writes subtasks for every listed member, and nothing reports that the project's primary crate was skipped. This is the same class as TASK-2105 (PATTERN-1, `parse_remote_url` silently dropping the port) — a value quietly disappears from an otherwise valid result.

There is also no test for the hybrid shape: `single_package_project_yields_the_root_package_as_the_only_target` covers `[package]` alone and `payload_lists_every_member_with_package_names` covers `[workspace]` alone.

**Why it matters**: `create-review-tasks` exists to guarantee a review subtask per crate. On a hybrid manifest the crate that holds most of the code is the one that never gets reviewed, and the run reports success. Note the sibling generic engine crate `extensions/create-review-tasks` (see TASK-2114) consumes this payload as ground truth and has no way to notice the omission.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A Cargo.toml declaring both [package] and a non-empty [workspace].members yields a review target for the root package in addition to every resolved member
- [ ] #2 The root-package target is emitted at path ROOT_PACKAGE_PATH (".") and is not duplicated when the root path also appears in the resolved member list
- [ ] #3 A test covers the hybrid shape (root [package] + [workspace] members) and asserts both the root package and every member appear exactly once
- [ ] #4 The existing pure-workspace and pure-single-package behaviours are unchanged and still covered by their tests
<!-- AC:END -->
