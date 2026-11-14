---
id: TASK-2635
title: 'Rust stack verify runs no doctests, so a broken doctest passes ops verify and fails in qa/CI'
status: Done
assignee: []
created_date: '2026-10-20 15:03'
updated_date: '2026-10-29 21:11'
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
- [x] #1 Default Rust verify fails on a doctest that does not compile (fixture workspace in an integration test)
- [x] #2 verify-fix gets the same leg, so the two keep the same coverage
- [x] #3 help text and docs/command-mappings.md list the new leg; docs/commands.md no longer implies doctests live only in qa
- [x] #4 Measured per-commit cost in the task notes (warm cache, ops repo and a mid-size workspace)

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Option (b) is not available: cargo 1.98 rejects `cargo test --doc --no-run` ("can't skip running doc tests with --no-run"), and rustdoc --no-run is unstable. Went with (a): `test-doc` added to the default Rust `verify` and `verify-fix` parallel stage (after the rewriters in verify-fix). No staging needed: the doctests link the dev-profile libs `build` produces, so in the parallel stage test-doc waits on cargo's build lock and recompiles nothing (checked on a fixture: `cargo test --doc` after `cargo build --all-targets` compiles no crate). Test: crates/cli/tests/integration.rs cli_verify_in_rust_project_fails_on_a_doctest_that_does_not_compile (fails on the old defaults, passes with the change); stack::tests reader list extended. Caveat: a package with no lib target fails `cargo test --doc` ("no library targets found") — already true of qa; documented in docs/command-mappings.md with the override. Cost, warm cache, 16 cores, 2026-10-29, load 1.4-6.4: ops repo (353 .rs) verify no-op 0.87s -> 3.6s, after touching crates/core/src/lib.rs 89-92s -> 93-94s; dbsec (236 .rs) no-op 8.5-8.8s -> 10.0-10.3s, after touching crates/core/src/lib.rs 34.6-34.8s -> 35.3-37.4s. Doctests are re-run every time (not cached), so the no-op case pays the full run.

CodeRabbit follow-up: `cargo test --doc` fails on a lib-less package, so crates/core/src/config/doctests.rs drops `test-doc` from the stack-default verify/verify-fix/qa/qa-next when the root Cargo.toml is a single [package] with no [lib] and no src/lib.rs (or autolib = false). Config-defined composites and a config-defined test-doc are left alone; workspaces keep the leg. Integration test cli_verify_in_bin_only_rust_project_skips_doctests_and_passes.

<!-- SECTION:NOTES:END -->
