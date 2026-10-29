---
id: TASK-2642
title: 'Rust stack: detect nested cargo workspaces and write them into [rust] workspaces on ops init'
status: Done
assignee: []
created_date: '2026-10-27 09:14'
updated_date: '2026-10-29 21:04'
labels:
  - feature
  - config
  - rust
dependencies:
  - TASK-2636
modified_files: []
priority: high
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
TASK-2636 adds a [rust] workspaces list that fans the workspace built-ins out per manifest. Writing that list by hand means each repo has to know it has a second workspace; ops can find it.

Cargo has no command that lists every workspace in a repository (cargo metadata only sees the one the cwd belongs to), but the manifests say enough: a Cargo.toml below the root with its own [workspace] table is a separate workspace (cargo-fuzz generates fuzz/Cargo.toml with [workspace] members = ["."] for this reason), and the root [workspace].exclude naming that directory confirms it. Walk the tree bounded, skipping target/, .git/, vendored and gitignored paths; a manifest without [workspace] that the root does not exclude is a member or a fixture, not a workspace. The member/exclude glob handling in extensions-rust/about/src/members.rs can be reused.

Detection runs at init and check time only, never per dispatch: the resolved plan should change when .ops.toml changes, not when a directory appears, and leaving a nested workspace out stays an explicit choice.

- ops init --commands on a Rust stack writes [rust] workspaces = [".", ...] with every detected workspace, each with a comment naming why it was picked, and prints the list it found.
- ops init --rust --check reports a detected workspace missing from the list, and a listed one that no longer exists, so repos with an existing .ops.toml (dbsec) learn about it without re-running init.

init stays non-interactive; an interactive picker can follow if writing-then-deleting proves awkward.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A nested Cargo.toml with its own [workspace] table is detected; one listed in the root [workspace].exclude is reported with that reason
- [x] #2 target/, .git/ and gitignored paths are not walked, and member or fixture manifests without [workspace] are not reported
- [x] #3 ops init --commands writes [rust] workspaces with the detected entries, commented, and prints them
- [x] #4 ops init --rust --check reports drift between the detected workspaces and the configured list in both directions
- [x] #5 Detection does not run on ordinary command dispatch
- [x] #6 docs/configuration.md describes detection and the init/check behaviour

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Detection is ops_cargo_toml::nested_workspaces (ignore-crate walk, depth 8). `ops init` writes [cargo] workspaces whenever a Rust project has nested workspaces, not only under --commands: the stack defaults run without --commands, so the setting matters either way. Foundation drift is reported per directory at `.ops.toml:cargo.workspaces.<dir>`, so a waiver can leave one out deliberately.

Shipped in PR #105 (feat/cargo-workspaces, v0.88.0); status closed 2026-10-29.

<!-- SECTION:NOTES:END -->
