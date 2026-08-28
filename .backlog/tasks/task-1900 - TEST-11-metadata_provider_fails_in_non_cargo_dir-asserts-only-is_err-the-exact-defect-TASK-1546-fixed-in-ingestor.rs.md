---
id: TASK-1900
title: >-
  TEST-11: metadata_provider_fails_in_non_cargo_dir asserts only is_err(), the
  exact defect TASK-1546 fixed in ingestor.rs
status: To Do
assignee:
  - TASK-1999
created_date: '2026-08-27 15:37'
updated_date: '2026-08-28 14:14'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions-rust/metadata/src/tests/wiring.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/tests/wiring.rs:34-40`

**What**:

```rust
#[test]
fn metadata_provider_fails_in_non_cargo_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut ctx = Context::test_context(dir.path().to_path_buf());
    let result = MetadataProvider.provide(&mut ctx);
    assert!(result.is_err());
}
```

`provide` reaches `try_provide_from_db` -> `provide_via_cargo_metadata` -> `run_cargo_metadata`, which can fail for at least four unrelated reasons: cargo is not on `PATH` (`RunError::Io`), the subprocess exceeded `CARGO_METADATA_TIMEOUT` on a loaded CI box (`RunError::Timeout`), the DuckDB path failed before cargo ran at all, or — the thing the test name promises — cargo ran and found no `Cargo.toml`. `is_err()` cannot tell them apart, so the test is green in every one of those environments and green for the wrong reason in three of them.

This is the identical defect that TASK-1546 fixed one file over. `ingestor.rs:161-188` now matches on `DbError::External` and asserts the chain mentions `cargo metadata`, with an explicit `panic!` arm for the `DbError::Io` case and a comment ("is `cargo` on PATH?") explaining why the bare assertion was inadequate. The fix was never carried across to the provider-level test that has the same exposure through the same subprocess call.

**Why it matters**: TEST-11. This test is the only coverage of `MetadataProvider::provide`'s failure path, and it currently certifies "something went wrong" — which is also true of a broken test harness. Its sibling `metadata_provider_returns_valid_json` (:23) depends on cargo being present, so an environment that loses cargo turns one test red and leaves this one green, which is precisely backwards as a diagnostic.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 metadata_provider_fails_in_non_cargo_dir asserts the specific failure variant and message, following the DbError::External + chain-contains-'cargo metadata' pattern established at ingestor.rs:161-188
- [ ] #2 The test fails loudly with a distinguishable message when cargo is absent from PATH or the subprocess times out, rather than passing
<!-- AC:END -->
