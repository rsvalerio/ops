---
id: TASK-2068
title: >-
  CONC-9: cargo llvm-cov's 15-minute timeout ignores the dispatch deadline,
  which TASK-2056 made shrinkable
status: Triage
assignee: []
created_date: '2026-08-29 18:10'
labels:
  - code-review-rust
  - concurrency
dependencies: []
modified_files:
  - extensions-rust/test-coverage/src/subprocess.rs
  - crates/extension/src/data.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/subprocess.rs:18`

**What**: `CARGO_LLVM_COV_TIMEOUT` is a fixed `Duration::from_mins(15)`, chosen
under the invariant documented on `ops_extension::DEFAULT_PROVIDER_BUDGET`:
the dispatch budget (20 min) "must stay **above** every subprocess timeout a
provider can wait on", or the budget fires first and reports a run still
within its own limit as a failure.

TASK-2056 made that budget operator-configurable via
`[data] provider_budget_secs`, so the invariant is now breakable from a
project's `.ops.toml`. An operator who tightens the budget to 60s to protect
against a slow network filesystem gets the coverage provider blocking in
`cargo llvm-cov` for the full 15 minutes and only *then* being told it was
over budget — precisely the "the bound is a label on the stall rather than a
cure for it" shape that SEC-33 / TASK-2052 just removed from the tree
walkers.

`Context::deadline()` already exists for exactly this case; its doc says
"Providers that hand work to something with its own timeout knob (an external
command, a database statement) can use this to size that timeout so the inner
wait cannot outlive the outer budget." The coverage provider is the one
in-tree caller that should be doing it and is not.

Sizing the subprocess timeout as `min(CARGO_LLVM_COV_TIMEOUT, time
remaining on ctx.deadline())` would make the two agree by construction and
retire the prose invariant, which no test enforces.

**Why it matters**: a configurable budget an in-tree provider ignores is worse
than a fixed one — the operator believes they bounded the stall and did not.
It also leaves `DEFAULT_PROVIDER_BUDGET`'s ordering requirement as a comment
that a config file can now violate silently.

**Origin**: discovered during TASK-2060 while fixing TASK-2056; the coverage
crate is outside that wave's file scope.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The cargo llvm-cov subprocess timeout is sized from the remaining Context deadline, never exceeding it
- [ ] #2 A test pins that a tightened provider budget shortens the subprocess wait rather than only reporting it afterwards
<!-- AC:END -->
