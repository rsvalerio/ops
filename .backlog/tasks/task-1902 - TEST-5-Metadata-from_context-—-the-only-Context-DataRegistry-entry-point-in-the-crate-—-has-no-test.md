---
id: TASK-1902
title: >-
  TEST-5: Metadata::from_context — the only Context/DataRegistry entry point in
  the crate — has no test
status: Triage
assignee: []
created_date: '2026-08-27 15:38'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions-rust/metadata/src/types.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/types.rs:205-214`

**What**: `from_context` is the sole function in `types.rs` that touches the extension framework, and nothing anywhere calls it — not production code, not the ~60 tests in `src/tests/`. Every test constructs its `Metadata` through `Metadata::from_value` or the `WsFixture::metadata()` helper, both of which take an owned `serde_json::Value` and bypass this function entirely.

Three documented behaviours are therefore unverified:

- **the `Arc` sharing claim.** The struct doc (`types.rs:114-118`) states `from_context` "can clone the cached pointer instead of deep-cloning the whole metadata blob". No test asserts that two `from_context` calls yield `Metadata` values whose `inner` are the same allocation (`Arc::ptr_eq`), which is the entire justification for holding `inner` as `Arc<Value>`.
- **the error type contract.** The doc comment cites "ERR-2 / TASK-1542: returns the framework's typed `DataProviderError` ... so downstream consumers can match on the failure variant (`NotFound`, `ComputationFailed`, `Serialization`, `Cycle`) without string-sniffing". Nothing exercises any of those variants through this function.
- **cache-lifetime semantics.** `types.rs:120-133` reasons at length about `OnceLock` caches being per-wrapper and advises callers to "hold the same `Metadata` value across those calls". No test pins that a fresh `from_context` yields empty caches.

`ops_extension::Context::test_context` already exists and is used by `wiring.rs` and `ingestor.rs`, so the seam for this test is in place.

**Why it matters**: TEST-5 — a public API function with no test anywhere in the suite. Note the interaction with the ARCH-9 finding on the same file: if the typed layer is removed as unconsumed, this becomes moot; if it is kept, this is the one function whose contract is stated in prose and checked nowhere.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test drives Metadata::from_context through a Context::test_context + DataRegistry with the metadata provider registered and asserts on the resulting workspace_root
- [ ] #2 A test asserts Arc::ptr_eq between the inner values of two from_context calls on the same Context, pinning the no-deep-clone claim in the struct doc
- [ ] #3 A test exercises at least one DataProviderError variant returned through from_context (e.g. the provider absent from the registry)
- [ ] #4 If the ARCH-9 finding on types.rs resolves as 'remove the unconsumed surface', this task is closed as obsolete rather than implemented
<!-- AC:END -->
