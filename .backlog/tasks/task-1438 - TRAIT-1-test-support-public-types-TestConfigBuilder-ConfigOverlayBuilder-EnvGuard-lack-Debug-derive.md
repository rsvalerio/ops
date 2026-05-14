---
id: TASK-1438
title: >-
  TRAIT-1: test-support public types (TestConfigBuilder, ConfigOverlayBuilder,
  EnvGuard) lack Debug derive
status: Done
assignee:
  - TASK-1460
created_date: '2026-05-13 18:33'
updated_date: '2026-05-14 09:09'
labels:
  - code-review-rust
  - trait
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/test_utils.rs:105,201,385`

**What**: Three `pub struct`s exposed via the `test-support` feature — `TestConfigBuilder` (line 105), `ConfigOverlayBuilder` (line 201), `EnvGuard` (line 385) — carry `#[allow(dead_code)]` but no `#[derive(Debug)]`. Downstream crates depending on `ops-core` with `features = ["test-support"]` consume these helpers in their own fixtures.

**Why it matters**: Rust API guideline C-DEBUG applies to feature-gated public APIs the same as to the default surface. A downstream test that wraps an `EnvGuard` inside a `Debug`-deriving fixture (RAII chain, parameterised test struct) cannot derive `Debug` on the wrapper, forcing boilerplate `impl Debug` solely to satisfy the bound. Note: `EnvGuard::original: Option<String>` may need a manual `Debug` impl if the captured env value should be redacted; the two builder types are pure derive candidates.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 TestConfigBuilder and ConfigOverlayBuilder derive Debug
- [ ] #2 EnvGuard either derives Debug or implements it manually (with redaction if appropriate)
- [ ] #3 cargo build -p ops-core --features test-support passes
<!-- AC:END -->
