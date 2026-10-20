---
id: TASK-2635
title: 'Rust stack verify runs no doctests, so a broken doctest passes ops verify and fails in qa/CI'
status: Triage
assignee: []
created_date: '2026-10-20 15:03'
labels:
  - rust
  - stack-defaults
  - verify
dependencies: []
modified_files:
  - crates/core/src/.default.rust.ops.toml
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The Rust stack default `[commands.verify]` (crates/core/src/.default.rust.ops.toml) runs fmt-check, whitespace/EOF checks, clippy, build, check-json, check-yaml and doc. None of these runs doctests: `build --all-targets` and clippy compile examples but not doc-comment code, and `doc` renders doctests without compiling them. Doctests only run in `test-doc`, which is in `qa` / `qa-next` alone.

Observed in dbsec (PR #106, 2026-10-18): a doctest in `crates/core/src/testing.rs` used `assert_matches!` without `# use std::assert_matches;`. `ops verify` passed 20/20 locally and the failure surfaced only in CI's `rust / qa` job, costing a push/CI round trip.

`verify` is the pre-commit gate and is meant to catch compile-level breakage. A doctest that does not compile is compile-level breakage in a doc comment, so it belongs there.

### Options
- (a) Add `test-doc` to the default `verify` (and `verify-fix`). Cost: one more compile under the test profile per commit; doctests also link each example, which is slow in big workspaces.
- (b) Add a cheaper compile-only leg, e.g. `cargo test --doc --no-run` (check it really compiles doctests on the supported toolchain: historically `--no-run` skipped them, since rustdoc compiles doctests at run time), as `test-doc-check` in `verify`. This catches non-compiling doctests without running them.
- (c) Keep `verify` as is, but say in its help/docs (docs/commands.md, command-mappings.md) that it does not cover doctests, and have `ops init` scaffold a commented `[extend.verify]` line for projects that want them.

Lean towards (b) if `--no-run` with `--doc` works on the supported toolchain, otherwise (a) behind `exclusive`/staging so it reuses `build`'s artifacts.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Default Rust verify fails on a doctest that does not compile (fixture workspace in an integration test)
- [ ] #2 verify-fix gets the same leg, so the two keep the same coverage
- [ ] #3 help text and docs/command-mappings.md list the new leg; docs/commands.md no longer implies doctests live only in qa
- [ ] #4 Measured per-commit cost in the task notes (warm cache, ops repo and a mid-size workspace)
<!-- AC:END -->
