---
id: TASK-002
title: "Shallow assertions in registry tests — missing output validation"
status: Triage
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-test-quality, TQ, TEST-11, medium, crate-cli]
dependencies: []
---

## Description

**Location**: `crates/cli/src/registry.rs`
**Anchor**: `mod tests`
**Impact**: Tests like `extension_info_provides_metadata` only validate fields are non-empty without checking actual extension data. `collect_compiled_extensions_returns_entries` checks names exist but not correctness. This provides weak guarantees against regressions in registry content.

**Notes**:
- TEST-11: Assert specific values, not just emptiness checks
- Fix: Assert expected extension names (e.g., "tools", "go", etc.) appear in the registry; verify metadata fields match known values
