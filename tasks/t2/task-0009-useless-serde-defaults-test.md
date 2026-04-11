---
id: TASK-0009
title: "Useless: default_true test is a tautology"
status: Triage
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-test-quality, TQ, TEST-1, low, crate-core]
dependencies: []
---

## Description

**Location**: `crates/core/src/serde_defaults.rs:7-end`
**Anchor**: `fn default_true_returns_true`
**Impact**: The test asserts that a `const fn` returning the literal `true` returns `true`. This is a tautology that adds no safety net — it would only fail if the compiler itself were broken. The function exists solely as a serde default, and is already exercised by config deserialization tests.

**Notes**:
Consider removing this test entirely to reduce suite noise. If kept, at minimum add a comment explaining it exists only as a serde contract marker. The real coverage for this default is in config deserialization tests that verify `show_error_detail` defaults to `true`.
