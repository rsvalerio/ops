---
id: TASK-024
title: "FrameworkOnly test in serde_defaults.rs — only tests const fn returns true"
status: Triage
assignee: []
created_date: '2026-04-09 00:00:00'
labels: [rust-test-quality, TQ, TEST-25, low, crate-core]
dependencies: []
---

## Description

**Location**: `crates/core/src/serde_defaults.rs:12-14`
**Anchor**: `fn default_true_returns_true`
**Impact**: The only test in this module asserts that `default_true()` returns `true`. This is a trivial `const fn` that always returns `true` — the test exercises no project logic and cannot meaningfully fail. It inflates test count without providing value.

**Notes**:
The function is used as a serde default (`#[serde(default = "default_true")]`). A more useful test would verify that the serde attribute works correctly by deserializing a struct with a missing field and checking the default is applied. Alternatively, the test can simply be removed since the const fn is trivially correct.
