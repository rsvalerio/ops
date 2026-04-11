---
id: TASK-025
title: "No tests for config/theme_types.rs public API"
status: Triage
assignee: []
created_date: '2026-04-09 00:00:00'
labels: [rust-test-quality, TQ, TEST-5, medium, crate-core]
dependencies: []
---

## Description

**Location**: `crates/core/src/config/theme_types.rs`
**Anchor**: (entire file)
**Impact**: `theme_types.rs` defines `ThemeConfig`, `ErrorBlockChars`, and `PlanHeaderStyle` — public types with constructor methods (`classic()`, `compact()`), defaults (`ErrorBlockChars::default()`), and a `status_icon()` method. None of these have direct unit tests. While `ThemeConfig::classic()` and `ThemeConfig::compact()` are exercised indirectly through `crates/theme/src/tests.rs`, the `status_icon()` method and `ErrorBlockChars::default()` values are not verified anywhere.

**Notes**:
Recommended tests:
- `ErrorBlockChars::default()` returns expected characters
- `ThemeConfig::status_icon()` maps each `StepStatus` variant to the correct icon
- `PlanHeaderStyle` enum variants deserialize correctly from TOML strings
