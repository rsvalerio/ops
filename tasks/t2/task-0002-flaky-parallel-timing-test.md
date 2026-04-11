---
id: TASK-0002
title: "Flaky wall-clock concurrency assertion in parallel timing test"
status: Triage
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-test-quality, TQ, TEST-13, high, crate-runner]
dependencies: []
---

## Description

**Location**: `crates/runner/src/command/tests.rs:978`
**Anchor**: `fn run_plan_parallel_executes_concurrently`
**Impact**: Asserts two real `sleep 1` processes complete in under 1.8s wall-clock. Under heavy CI load, the 0.8s margin may be insufficient, causing false failures.

**Notes**:
This test verifies parallelism by measuring wall-clock elapsed time. No `tokio::time::pause()` or mock clock is used. The 0.8s margin assumes the CI runner can schedule both processes within 0.8s of each other, which is not guaranteed on overloaded or single-core VMs. Consider testing the concurrency property structurally (e.g., verifying interleaved event ordering) rather than via timing.
