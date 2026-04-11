---
id: TASK-0011
title: "Test gap: Stack::resolve() config override path untested"
status: Triage
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-test-quality, TQ, TEST-6, medium, crate-core]
dependencies: []
---

## Description

**Location**: `crates/core/src/stack.rs`
**Anchor**: `fn resolve`
**Impact**: `Stack::resolve()` has a `config_stack` override path that is never tested. Only `Stack::detect()` (filesystem-based detection) is covered. When a user explicitly sets `stack = "rust"` in config, `resolve()` should prefer it over detection — but this behavior has no test.

**Notes**:
Add tests for: (1) `resolve()` with `config_stack = Some("rust")` returns `Rust` regardless of filesystem, (2) `resolve()` with `config_stack = None` falls through to `detect()`, (3) `resolve()` with an invalid `config_stack` value.
