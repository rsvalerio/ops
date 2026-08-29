---
id: TASK-2030
title: >-
  TEST-31: ops deps is never run as a CLI subcommand — its exit code and
  stdout/stderr routing are untested
status: Triage
assignee: []
created_date: '2026-08-28 20:52'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - crates/cli/src/subcommands.rs
  - crates/cli/src/main.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/subcommands.rs:61-65` (`run_deps`), `crates/cli/src/main.rs` (the `deps` arm)

**What**: TASK-1845 closed the library-level gap — `tests::command_path_tests` now drives `ops_deps::run_deps`, `ensure_tools` and `DepsProvider::provide` in-process. What it deliberately did not cover is the subcommand itself: nothing spawns the `ops` binary with `deps` and asserts what a user or CI actually observes.

Three things live only in the CLI layer and are asserted nowhere:

- **Exit code.** `run_deps` returns `Err("dependency issues found")`; whether the binary turns that into a non-zero exit — and which code — is `main.rs`'s job. "`ops deps` fails CI when there are dependency issues" is the product's contract and it is pinned only up to the `anyhow::Error`.
- **Stream routing.** The report is written to stdout with `println!` while errors go through the CLI's error path to stderr. A change that routed the report to stderr (or the error to stdout) would break every pipeline consuming `ops deps` and pass the whole suite.
- **`--refresh` plumbing.** `crates/cli/src/subcommands.rs:63` builds `DepsOptions::new(refresh)` from the parsed flag. The library test pins `DepsOptions.refresh -> ctx.refresh`; nothing pins `--refresh -> DepsOptions`.

The same argument applies to the sibling subcommands wired through `cli_data_context`, so a shared harness is probably worth more than a `deps`-only test.

**Why it matters**: every hardening task in the deps crate's history protects one property — `ops deps` must fail loudly rather than score green — and all of them are pinned below the CLI boundary. The boundary is where "fails loudly" finally becomes an exit code, and it is the last unpinned link in that chain.

**Origin**: discovered during TASK-1997 while fixing TASK-1845, whose description raises TEST-31 but whose acceptance criteria scope it to the library surface.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test runs the built ops binary with the deps subcommand against a fixture project and asserts a non-zero exit when the report carries actionable issues
- [ ] #2 The same harness asserts a zero exit on a clean report
- [ ] #3 The test asserts the rendered report lands on stdout and the failure message on stderr
- [ ] #4 --refresh is asserted to reach DepsOptions
- [ ] #5 The test does not require real cargo-edit / cargo-deny installations and does not reach the network
<!-- AC:END -->
