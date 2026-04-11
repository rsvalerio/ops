---
id: TASK-0015
title: "Useless: run_extension_list test is a pure smoke test"
status: Triage
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-test-quality, TQ, TEST-11, low, crate-cli]
dependencies: []
---

## Description

**Location**: `crates/cli/src/extension_cmd.rs:310-end`
**Anchor**: `fn run_extension_list_succeeds`
**Impact**: Only asserts `is_ok()` with no output inspection. The test would pass even if the list output were empty or garbled. Similarly, `run_extension_show_tools_succeeds` only asserts `is_ok()`.

**Notes**:
These tests should capture the writer output and verify that extension names appear. The other tests in this module (`run_extension_show_unknown_returns_error`, `format_list_*`) have meaningful assertions, so these two are outliers. Low severity because extension listing is not critical path.
