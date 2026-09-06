---
id: TASK-1800
title: >-
  TEST-11: extension_with_root_propagates_to_provider asserts nothing about the
  root it claims to propagate
status: Done
assignee:
  - TASK-1994
created_date: '2026-08-27 11:25'
updated_date: '2026-08-28 20:19'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions-rust/cargo-toml/src/tests/extension.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/tests/extension.rs:10-28`.

**What**: the test builds a tempdir, writes a `Cargo.toml` containing `name = "test-crate"`, constructs `CargoTomlExtension::with_root(temp_dir.path())`, registers the data providers, then asserts:

```rust
let provider = registry.get("cargo_toml").expect("provider registered");
assert_eq!(provider.name(), "cargo_toml");
```

`provider.name()` returns the `DATA_PROVIDER_NAME` constant (`lib.rs:240-242`) — it is the same value whichever root was passed, and the same value `CargoTomlExtension::new()` would register. The written manifest is never read. The whole `with_root` path under test (`lib.rs:145-151`, the `map_or_else(CargoTomlProvider::new, |p| CargoTomlProvider::with_root(p.clone()))` arm) could be replaced with the unconditional `CargoTomlProvider::new()` branch and this test would still pass.

**Why it matters**: TEST-11 — assert the actual result, not that a call succeeded. The `with_root` override is the mechanism by which `extensions-rust/about` and `extensions-rust/create-review-tasks` pin the provider to a root they resolved themselves (`create-review-tasks/src/provider.rs:24`), so a silent regression in that wiring would send both to auto-discovery from the working directory instead — the failure mode TASK-0501 documents as "silently produced empty units/coverage". The test's name promises exactly this coverage and does not deliver it, which is the "creates false confidence" case in the classification guide.

The fix is available without new machinery: the sibling test `provider_resolve_root_auto_discovers` (`src/tests/provider.rs:114-133`) already shows the pattern — call `provide` through a `Context` and assert `manifest.package_name()`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 extension_with_root_propagates_to_provider drives the registered provider (e.g. via provide/provide_typed with a Context whose working_directory is unrelated to the root) and asserts it parsed the manifest at the configured root — package_name() == Some("test-crate")
- [x] #2 A companion assertion shows the override actually overrides: with a Context working_directory pointing somewhere else, the configured root still wins
- [x] #3 The written Cargo.toml fixture is load-bearing rather than dead setup
<!-- AC:END -->
