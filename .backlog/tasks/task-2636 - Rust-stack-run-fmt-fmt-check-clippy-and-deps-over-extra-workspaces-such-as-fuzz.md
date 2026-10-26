---
id: TASK-2636
title: 'Rust stack: run fmt, fmt-check, clippy and deps over extra workspaces such as fuzz/'
status: To Do
assignee: []
created_date: '2026-10-26 22:12'
labels:
  - feature
  - config
  - rust
dependencies: []
modified_files: []
priority: high
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A repository with a second cargo workspace beside the root (cargo-fuzz's fuzz/, excluded from [workspace]) gets none of the Rust built-ins over it: fmt, fmt-check and clippy are --workspace/--all, and deps runs cargo deny over the root lockfile. Each repo re-adds them by hand.

dbsec's .ops.toml carries seven definitions for this alone: fuzz-fmt, fuzz-fmt-check and fuzz-clippy (clones with --manifest-path fuzz/Cargo.toml), fuzz-deny, fmt-workspace plus a redefined fmt composite (so verify-fix formats both), a deny composite (deps + fuzz-deny), and fuzz-deny appended to its qa.

Proposal: a [rust] setting naming the workspaces, e.g. workspaces = [".", "fuzz"]. Built-ins that act on a workspace (fmt, fmt-check, clippy, check, build, deps --check) expand to one step per manifest, the way a strategy matrix expands, labelled by workspace. Test commands probably stay root-only, or opt in.

Phase 2, maybe: options for the drift between a nested workspace and the root, which dbsec checks with ~490 lines of script: the nested Cargo.lock must agree with the root one for shared crates (scripts/fuzz-lock-check.sh), and the nested [lints] tables must copy [workspace.lints] (scripts/lint-sync-check.sh).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A configured extra workspace gets fmt, fmt-check, clippy and deps steps without any [commands] entry
- [ ] #2 ops explain verify shows one labelled step per workspace
- [ ] #3 dbsec's fuzz-fmt, fuzz-fmt-check, fuzz-clippy, fuzz-deny, fmt-workspace, fmt and deny definitions can be deleted with an unchanged resolved plan
- [ ] #4 docs/configuration.md and command-mappings.md describe the setting
<!-- AC:END -->
