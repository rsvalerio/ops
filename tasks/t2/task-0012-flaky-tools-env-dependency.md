---
id: TASK-0012
title: "Flaky: tools_cmd tests depend on cargo-fmt being installed"
status: Triage
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-test-quality, TQ, TEST-17, medium, crate-cli]
dependencies: []
---

## Description

**Location**: `crates/cli/src/tools_cmd.rs:203-end`
**Anchor**: `fn tools_check_all_installed`, `fn tools_install_all_present`, `fn tools_install_specific_already_installed`
**Impact**: Three tests implicitly depend on `cargo-fmt` being installed in the test environment. They will fail in minimal CI environments or containers without the full Rust toolchain.

**Notes**:
These tests use `cargo-fmt` as the "known installed tool" to verify list/check/install behavior. If the CI image doesn't have `cargo-fmt`, the tests fail with misleading "tool not installed" results rather than actual logic failures. Consider using a tool guaranteed to exist (e.g., `sh`, `echo`) or mocking the `which` check to isolate the test from the environment.
