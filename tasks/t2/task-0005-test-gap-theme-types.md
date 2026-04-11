---
id: TASK-0005
title: "Test gap: ThemeConfig::status_icon() has no per-variant coverage"
status: Triage
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-test-quality, TQ, TEST-7, medium, crate-core]
dependencies: []
---

## Description

**Location**: `crates/core/src/config/theme_types.rs`
**Anchor**: `fn status_icon`
**Impact**: `ThemeConfig::status_icon()` maps 5 `StepStatus` variants to icon strings. None of the 5 branches is directly tested. The function is exercised indirectly by theme rendering tests in `crates/theme`, but no unit test validates the mapping for all statuses on a `ThemeConfig` directly.

**Notes**:
Also untested in this file: `ErrorBlockChars::default()` field values and `PlanHeaderStyle` serde round-trip. The `classic()` and `compact()` factory methods are `#[cfg(test)]`-gated and used as test fixtures elsewhere but have no dedicated tests verifying their field values. The theme rendering tests in `crates/theme/src/tests.rs` provide indirect coverage for `classic`/`compact` icon values via `resolve_theme_classic` and `resolve_theme_compact`, which partially mitigates this gap.
