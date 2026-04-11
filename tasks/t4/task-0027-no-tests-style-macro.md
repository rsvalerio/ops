---
id: TASK-027
title: "No tests for ansi_style! macro in core/style.rs"
status: Triage
assignee: []
created_date: '2026-04-09 00:00:00'
labels: [rust-test-quality, TQ, TEST-5, low, crate-core]
dependencies: []
---

## Description

**Location**: `crates/core/src/style.rs:3-10`
**Anchor**: `macro_rules! ansi_style`
**Impact**: The `ansi_style!` macro is a public utility for generating ANSI escape sequences. It has no tests. The macro is likely exercised indirectly through theme rendering, but a direct test would catch regressions in escape code generation.

**Notes**:
Low severity because the macro is small and likely exercised through downstream usage. A simple test verifying the output contains expected ANSI escape codes would suffice. Consider whether the macro is actually `pub` and used outside the crate — if it's internal-only and tested via theme rendering, this gap is acceptable.
