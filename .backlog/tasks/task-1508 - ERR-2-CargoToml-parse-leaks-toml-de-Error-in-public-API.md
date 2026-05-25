---
id: TASK-1508
title: 'ERR-2: CargoToml::parse leaks toml::de::Error in public API'
status: Done
assignee:
  - TASK-1642
created_date: '2026-05-18 19:15'
updated_date: '2026-05-25 16:21'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/types.rs:60`

**What**: `pub fn parse(content: &str) -> Result<Self, toml::de::Error>` exposes the `toml` crate's error type directly in the library's public API. Every consumer of `CargoToml::parse` is now compile-coupled to the exact major version of `toml` that ops-cargo-toml depends on; a `toml` 0.8 → 0.9 bump that renames or restructures `de::Error` becomes a breaking change for every downstream extension that pattern-matches on the returned error.

Contrast with the `provide_typed` path (lib.rs:230), which deliberately wraps the same parse in `anyhow::Context` so the public surface only sees `anyhow::Error` / `DataProviderError`.

**Why it matters**: ERR-2 (avoid leaking external error types in your public API). Once the API is published — even as a workspace-internal crate that other extensions consume — a `toml` crate bump cannot be done atomically: every callsite that names `toml::de::Error` must be updated in lockstep. The fix is either to wrap in a thiserror enum local to this crate (e.g. `CargoTomlParseError`) or to return `anyhow::Error` from `parse` and let consumers downcast if they need the underlying TOML span.

The doctest at types.rs:23 (`CargoToml::parse(toml_content).unwrap()`) suggests no consumer currently inspects the error variant, so the wrapping change should be source-compatible for normal `?`-propagating callers.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 CargoToml::parse returns a crate-local error type or anyhow::Error, not toml::de::Error directly
- [ ] #2 Existing call sites continue to compile (?, anyhow conversion still works) without each having to import toml::de::Error
<!-- AC:END -->
