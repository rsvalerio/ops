---
id: TASK-2178
title: >-
  PATTERN-1: go.mod local `replace` targets are counted as modules, so the About
  card's module count disagrees with the units list
status: To Do
assignee:
  - TASK-2239
created_date: '2026-09-08 07:12'
updated_date: '2026-09-08 10:55'
labels:
  - code-review-rust
  - pattern
dependencies: []
modified_files:
  - extensions-go/about/src/lib.rs
  - extensions-go/about/src/modules.rs
priority: medium
ordinal: 91000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/lib.rs:104` (`compute_module_count`), consumed by `extensions-go/about/src/modules.rs:29` (`collect_units`)

**What**: For a non-workspace project (`go.mod`, no `go.work`), `compute_module_count` returns `Some(1 + m.local_replaces.len())` — the root module plus every local `replace` target. The `project_units` provider on the very same run takes the other branch of `collect_units` and emits exactly **one** unit for that project (the root module, path `"."`); local `replace` targets are never turned into units.

The result is that a `go.mod` with two local replaces renders an identity card reading `modules: 3` (`crates/core/src/project_identity/card.rs:81`) above a units table with a single row. `lib.rs` even has a test pinning the divergent value (`provide_go_project_with_local_replaces` asserts `module_count == Some(3)`), and `modules.rs::collect_units_single_mod` asserts one unit — the two are never compared.

This also diverges from the cross-stack convention: `extensions-rust/about/src/identity/mod.rs:75` sets `module_count` from `manifest.resolved_members().len()`, i.e. exactly the set the units provider lists; Node and Python set `module_count = None` for single-package projects. Go is the only stack where the count and the list count different things.

Semantically a `replace` directive is a dependency substitution, not a workspace member. `go.work` `use` directives are the workspace-member concept, and that branch already agrees with the units list one-for-one.

**Why it matters**: The headline number on the About card is wrong for any Go repo that vendors a local fork via `replace` (a very common layout — the crate's own tests use `github.com/openbao/openbao` for this). A user reading "3 modules" next to a one-row module table has no way to reconcile the two, and the count grows with unrelated dependency-pinning edits.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 compute_module_count and collect_units agree on what counts as a module for every input: for any fixture, module_count is either None or equal to collect_units(...).len()
- [ ] #2 A go.mod-only project with local replace directives no longer reports a module count larger than the number of emitted ProjectUnits (either the replaces become units, or the count drops the replaces)
- [ ] #3 A test asserts the identity module_count and the units-provider length together on a single fixture containing local replaces
- [ ] #4 The chosen semantics are documented on compute_module_count and cross-referenced against the Rust stack's resolved_members() convention
<!-- AC:END -->
