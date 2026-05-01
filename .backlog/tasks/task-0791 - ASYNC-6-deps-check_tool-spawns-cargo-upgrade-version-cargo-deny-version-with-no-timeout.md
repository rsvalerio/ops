---
id: TASK-0791
title: >-
  ASYNC-6: deps::check_tool spawns cargo upgrade --version / cargo deny
  --version with no timeout
status: Triage
assignee: []
created_date: '2026-05-01 05:59'
labels:
  - code-review-rust
  - async
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/lib.rs:122-139`

**What**: check_tool calls Command::new("cargo").args(tool.probe_args).status() directly, bypassing ops_core::subprocess::run_cargo (and its OPS_SUBPROCESS_TIMEOUT_SECS deadline). Sibling subprocess invocations (run_cargo_upgrade_dry_run, run_cargo_deny) wrap calls in run_cargo with explicit timeouts; the probe path is the only spawn that can hang indefinitely.

**Why it matters**: cargo resolution can stall on a wedged registry probe, broken sccache wrapper, or sibling cargo build blocking on target/ lock. A hung probe freezes ops deps before it gets to do any work.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Route the probe through run_cargo (or an equivalent bounded wrapper) so it inherits the standard timeout + $CARGO resolution
- [ ] #2 Ensure a hung cargo --version/deny --version fails the probe with a clear timeout error instead of blocking the process
- [ ] #3 Add a unit test driving the timeout path (mock or sleep-based subprocess substitute) to lock the contract in
<!-- AC:END -->
