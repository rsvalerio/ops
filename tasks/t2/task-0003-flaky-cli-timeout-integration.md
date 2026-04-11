---
id: TASK-0003
title: "Flaky wall-clock timeout in CLI integration test"
status: Triage
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-test-quality, TQ, TEST-13, medium, crate-cli]
dependencies: [TASK-0001]
---

## Description

**Location**: `crates/cli/tests/integration.rs`
**Anchor**: `fn cli_run_command_with_timeout`
**Impact**: Integration test using real `sleep 3` with a 1-second timeout_secs config value. Same wall-clock timing fragility as TASK-0001 but at the CLI integration level.

**Notes**:
This is the integration-level counterpart of TASK-0001. Since integration tests spawn real processes, mock clocks are not applicable here. Consider increasing the safety margin or marking as `#[ignore]` with instructions for manual execution, similar to `run_command_cli_full_lifecycle`.
