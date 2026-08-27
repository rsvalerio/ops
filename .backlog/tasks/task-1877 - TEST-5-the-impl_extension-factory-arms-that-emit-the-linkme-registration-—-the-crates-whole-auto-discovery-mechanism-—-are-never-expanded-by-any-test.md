---
id: TASK-1877
title: >-
  TEST-5: the impl_extension! factory arms that emit the linkme registration —
  the crate's whole auto-discovery mechanism — are never expanded by any test
status: Triage
assignee: []
created_date: '2026-08-27 15:32'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - crates/extension/src/tests.rs
  - crates/extension/src/macros.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/macros.rs:59-88` and `crates/extension/src/macros.rs:118-143` (the two `factory:` arms); `crates/extension/src/tests.rs:573-623` (the only two `impl_extension!` expansions in the crate)

**What**: `impl_extension!` has four arms. The two that matter most — the ones carrying `factory: $ident = $expr` and emitting

```rust
#[linkme::distributed_slice($crate::EXTENSION_REGISTRY)]
static $factory_ident: $crate::ExtensionFactory = $factory_fn;
```

— are the entire compiled-in extension auto-discovery mechanism this crate owns. Neither is expanded anywhere in `crates/extension/src/tests.rs`. `MacroTestExtFull` uses the legacy full form (no factory) and `MacroTestExtShort` uses the legacy short form (no factory). `EXTENSION_REGISTRY` itself is never read in a test in this crate, and `ExtensionFactory` is never constructed.

The same gap covers the rest of the crate's macro and public surface:

- `test_datasource_extension!` (`macros.rs:206-220`) — the macro every downstream extension crate uses for its registration tests — is never invoked here, so a syntax or path regression in it breaks N downstream crates at once with no local signal.
- `DuckDbHandle` and its blanket impl (`data.rs:363-377`) have zero coverage; the `duckdb` feature is never enabled in a test run, so the downcast contract the rustdoc describes is unverified.
- Untested public API on the registries and context: `Context::clear_provider_results` (the ARCH-9 / TASK-1128 cache-invalidation hook), `Context::from_cwd_arc` (and the `Arc` sharing it exists to provide), `CommandRegistry::insert`'s returned previous value, `CommandRegistry::take_duplicate_inserts` on the direct-insert path (only the `FromIterator` path is covered), `IntoIterator for &CommandRegistry`, `CommandRegistry::clone` (whose hand-written impl exists specifically to reset the audit trail — TRAIT-4 / TASK-0653 — and is unverified), and `ExtensionInfo::new`.

`CommandRegistry::clone` is worth calling out: it is a hand-rolled impl whose entire reason to exist is a behavioural difference from `derive(Clone)`, and nothing pins that behaviour.

**Why it matters**: a macro arm that no test expands is code the compiler never sees until a downstream crate is built. A change to `ExtensionFactory`'s signature, to `EXTENSION_REGISTRY`'s type, or to the arm's token matching compiles fine in `cargo test -p ops-extension` and fails across every extension crate in the workspace. Since this crate is the framework, its tests are the only place that can catch such a regression at its source. `#[linkme::distributed_slice]` statics declared inside a `#[cfg(test)]` module are collected into the test binary's slice, so testing the factory arms end-to-end is straightforward.

**Suggested fix**: add a test-only extension in `tests.rs` for each of the two `factory:` arms, then assert `EXTENSION_REGISTRY` contains a factory that constructs it (iterate the slice, call each factory with an empty `Config` and a temp dir, and assert the expected `(config_name, Extension::name())` pair appears). Invoke `test_datasource_extension!` against a stub. Add the missing tests for `clear_provider_results`, `from_cwd_arc`, `CommandRegistry::insert`/`take_duplicate_inserts`/`Clone`, and `ExtensionInfo::new`. Add a `--features duckdb` job (or a `#[cfg(feature = "duckdb")]` test) covering the `DuckDbHandle` blanket impl and downcast.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Both impl_extension! factory arms are expanded by a test in crates/extension/src/tests.rs and the resulting factory is found by walking EXTENSION_REGISTRY
- [ ] #2 test_datasource_extension! is invoked at least once inside this crate so a regression in it fails here rather than in downstream extension crates
- [ ] #3 CommandRegistry::clone is covered by a test asserting the data is copied and the duplicate_inserts audit trail is reset, pinning the hand-written impl's reason to exist
- [ ] #4 Context::clear_provider_results, Context::from_cwd_arc, CommandRegistry::insert's return value, CommandRegistry::take_duplicate_inserts on the direct path, IntoIterator for &CommandRegistry and ExtensionInfo::new each have at least one test
- [ ] #5 The duckdb-feature surface (DuckDbHandle blanket impl and the documented downcast) is exercised under a feature-enabled test run
<!-- AC:END -->
