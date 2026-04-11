---
id: TASK-0008
title: "Weak OR assertion in classic_theme_running_status test"
status: Triage
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-test-quality, TQ, TEST-11, low, crate-theme]
dependencies: []
---

## Description

**Location**: `crates/theme/src/tests.rs`
**Anchor**: `fn classic_theme_running_status`
**Impact**: Uses `line.starts_with("◆ cargo test") || line.contains("cargo test")` — the OR makes this assertion trivially true for any output containing "cargo test". It does not verify the Running status icon or line structure.

**Notes**:
The intent is to verify the Running status renders with the correct icon and label. The OR branch was likely added because the exact prefix depends on runtime state (animation frame). If the running icon is non-deterministic, test it with a fixed icon or assert only the deterministic parts. At minimum, remove the `contains` fallback and use the specific prefix assertion.
