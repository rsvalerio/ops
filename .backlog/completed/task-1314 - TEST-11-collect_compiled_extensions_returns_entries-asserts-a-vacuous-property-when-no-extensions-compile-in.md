---
id: TASK-1314
title: >-
  TEST-11: collect_compiled_extensions_returns_entries asserts a vacuous
  property when no extensions compile in
status: Done
assignee:
  - TASK-1383
created_date: '2026-05-11 20:26'
updated_date: '2026-05-12 23:16'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry/tests.rs:101`

**What**: The test loops over `compiled` and asserts each entry's `name` is non-empty:

```rust
#[test]
fn collect_compiled_extensions_returns_entries() {
    let config = Config::default();
    let compiled = collect_compiled_extensions(&config, std::path::Path::new("."));
    for (name, ext) in &compiled {
        assert!(!name.is_empty());
        assert!(!ext.name().is_empty());
    }
}
```

When the crate is built with `--no-default-features` (or any feature set in which no extensions are compiled in), `compiled` is empty and the loop body executes zero times — the test passes trivially without exercising the function under test. The test name (`..._returns_entries`) implies that entries are returned, but nothing here asserts that, and the sibling test `collect_compiled_extensions_unfiltered_by_config` (line 111) even confirms the empty case is expected on some builds.

**Why it matters**: A test that can never fail on its target build profile gives a false sense of coverage and lets regressions in `collect_compiled_extensions` reach default-feature CI undetected. Either the test should be gated on a feature that guarantees at least one extension (e.g. `#[cfg(feature = "stack-rust")]`) and assert `!compiled.is_empty()`, or it should be re-scoped to verify a behavioural property that holds for any feature set (e.g. that `Config::default()` does not panic and the returned names are unique).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Test either asserts a non-vacuous property on every build (e.g. uniqueness, deterministic ordering, no-panic contract) OR is gated on a feature flag that guarantees compiled-in extensions and asserts !compiled.is_empty()
- [ ] #2 Under default features (no stack-*), the test still fails when collect_compiled_extensions regresses (e.g. starts panicking, returns duplicates, or yields empty names from compiled-in entries) — verified by mutation/manual injection
<!-- AC:END -->
