---
id: TASK-004
title: "Missing error-path tests for config/tools.rs parsing"
status: Triage
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-test-quality, TQ, TEST-6, low, crate-core]
dependencies: []
---

## Description

**Location**: `crates/core/src/config/tools.rs:62-96`
**Anchor**: `mod tests`
**Impact**: Tests only cover happy-path parsing of tool specs. No tests for invalid TOML, missing required `description` field, or invalid `source` enum value. Errors in deserialization logic would go undetected.

**Notes**:
- TEST-6: Test error paths and edge cases, not just happy paths
- Fix: Add tests for `parse_tool_spec("")` (empty), missing description field in extended format, invalid source value (e.g., `source = "invalid"`), and completely malformed TOML
