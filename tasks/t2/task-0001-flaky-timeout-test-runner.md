---
id: TASK-0001
title: "Flaky wall-clock timeout test in runner command tests"
status: Triage
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-test-quality, TQ, TEST-13, high, crate-runner]
dependencies: []
---

## Description

**Location**: `crates/runner/src/command/tests.rs:105`
**Anchor**: `fn run_exec_timeout`
**Impact**: Uses real `sleep 3` process with a 1-second wall-clock timeout. Under heavy CI load or single-core VMs, the tokio timer may not fire reliably within 1s of wall time. This creates intermittent CI failures that erode trust.

**Notes**:
The test spawns a real `sleep 3` process and asserts the runner's timeout mechanism kills it within 1s. While a 3x safety margin is documented, this is a real-time test without `tokio::time::pause()`. The async timeout depends on the tokio runtime scheduling the timer callback promptly, which is not guaranteed under CPU starvation. Consider using `tokio::time::pause()` with a mock clock or restructuring to avoid wall-clock assertions.
