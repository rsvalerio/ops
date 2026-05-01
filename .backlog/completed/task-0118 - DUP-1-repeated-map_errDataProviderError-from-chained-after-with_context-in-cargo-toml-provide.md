---
id: TASK-0118
title: >-
  DUP-1: repeated map_err(DataProviderError::from) chained after with_context in
  cargo-toml provide()
status: Done
assignee: []
created_date: '2026-04-19 18:41'
updated_date: '2026-04-19 19:42'
labels:
  - rust-code-review
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/lib.rs:191-206`

**What**: `provide()` repeats `.with_context(...).map_err(DataProviderError::from)` three times for separate fallible steps, plus a fourth `.map_err(DataProviderError::from)` on the final `to_value`.

**Why it matters**: Minor readability tax and invites drift if a future branch forgets the error conversion; a small helper (e.g. `fn to_provider_err<T>(r: anyhow::Result<T>) -> Result<T, DataProviderError>`) or an `impl From<anyhow::Error> for DataProviderError` with `?` directly would remove the boilerplate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 map_err(DataProviderError::from) boilerplate is removed in cargo-toml provide() (and similar providers)
- [x] #2 all provider tests still pass
<!-- AC:END -->
