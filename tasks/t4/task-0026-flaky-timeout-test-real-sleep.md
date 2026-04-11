---
id: TASK-026
title: "Flakiness risk in run_exec_timeout — real sleep with wall-clock timing"
status: Triage
assignee: []
created_date: '2026-04-09 00:00:00'
labels: [rust-test-quality, TQ, TEST-15, low, crate-runner]
dependencies: []
---

## Description

**Location**: `crates/runner/src/command/tests.rs:134-150`
**Anchor**: `fn run_exec_timeout`
**Impact**: This test spawns a real `sleep 3` process with a 1s timeout to verify timeout behavior. The 3x safety margin is documented (TQ-001 comment), and the trade-off is acknowledged. However, under extreme CI load, process startup overhead alone could exceed the 1s timeout before `sleep` even begins, causing a false failure. The same pattern appears in `cli_run_command_with_timeout` in `integration.rs:317-344`.

**Notes**:
The test already has thorough documentation explaining the trade-off. The risk is low given the 3x margin. A deterministic alternative would use a mock executor or `tokio::time::pause()` with a virtual clock, but the current approach is pragmatic for a test that exercises real subprocess timeout handling. No immediate fix needed — track for investigation if this test becomes flaky in CI.
