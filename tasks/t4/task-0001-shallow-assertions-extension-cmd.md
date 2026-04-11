---
id: TASK-001
title: "Shallow assertions in extension_cmd tests — is_ok() without output validation"
status: Triage
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-test-quality, TQ, TEST-11, medium, crate-cli]
dependencies: []
---

## Description

**Location**: `crates/cli/src/extension_cmd.rs`
**Anchor**: `mod tests`
**Impact**: Several tests assert only `is_ok()` or `is_err()` without verifying the actual output or error content. This reduces confidence that the extension commands produce correct results — a regression could change output entirely while tests still pass.

**Notes**:
- `run_extension_list_succeeds` checks `is_ok()` but never validates that extensions actually appear in output
- `run_extension_show_unknown_returns_error` checks the error message but not detailed validation
- TEST-11: Assert specific values, not just `is_ok()` / `is_some()`
- Fix: Capture stdout/stderr and assert expected extension names appear in list output; verify error messages contain the unknown extension name
