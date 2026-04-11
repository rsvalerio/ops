---
id: TASK-0006
title: "Test gap: style.rs 8 public functions with zero coverage"
status: Triage
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-test-quality, TQ, TEST-5, low, crate-core]
dependencies: []
---

## Description

**Location**: `crates/core/src/style.rs:1-end`
**Anchor**: `fn cyan`, `fn white`, `fn grey`, `fn dim`, `fn green`, `fn red`, `fn yellow`, `fn bold`
**Impact**: All 8 ANSI color/style wrapper functions are untested. Each wraps a string in escape codes (`\x1b[Nm...\x1b[0m`).

**Notes**:
These are simple one-liner functions with no branching. The risk is low — the only failure mode is a typo in escape codes. However, they are public API with zero coverage. A single parameterized test covering all 8 functions would close this gap with minimal effort. Severity is low because the functions are trivial and exercised indirectly through rendering tests.
