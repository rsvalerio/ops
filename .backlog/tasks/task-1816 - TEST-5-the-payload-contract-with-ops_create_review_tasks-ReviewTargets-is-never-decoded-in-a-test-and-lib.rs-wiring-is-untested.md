---
id: TASK-1816
title: >-
  TEST-5: the payload contract with ops_create_review_tasks::ReviewTargets is
  never decoded in a test, and lib.rs wiring is untested
status: Triage
assignee: []
created_date: '2026-08-27 11:32'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions-rust/create-review-tasks/src/provider.rs
  - extensions-rust/create-review-tasks/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/create-review-tasks/src/provider.rs:83-189`, `extensions-rust/create-review-tasks/src/lib.rs:1-46`

**What**: The entire purpose of this crate is to emit a JSON payload that the generic engine decodes into `ops_create_review_tasks::ReviewTargets` (`extensions/create-review-tasks/src/lib.rs:48-64`, decoded at lib.rs:149-150 with `serde_json::from_value`). Every test in `provider.rs` asserts against a hand-written `serde_json::json!` literal instead, so nothing in this crate's test suite ever constructs the consumer's type. This crate already depends on `ops-create-review-tasks` (Cargo.toml:14) and `ReviewTargets` / `ReviewTarget` are `pub`, so the round-trip is one line away.

Untested public surface, in the same file pair:

- `RustReviewTargetsProvider::name()` (provider.rs:17-19) — no test asserts it equals `ops_create_review_tasks::DATA_PROVIDER_NAME`; a provider registered under the wrong key silently becomes a `DataProviderError::NotFound` at runtime, which the engine reports as "the detected stack has no create-review-tasks extension compiled in".
- `lib.rs` in its entirety — `NAME`, `SHORTNAME`, `DATA_PROVIDER_NAME` and the `register_data_providers` closure that registers the provider have no test at all. The `#![cfg_attr(test, allow(...))]` block at lib.rs:8-16 is boilerplate over a file with zero tests.

**Why it matters**: the producer and the consumer of this payload live in different crates and are joined only by `serde_json::Value`, so a field rename or a shape change on `ReviewTargets` compiles cleanly on both sides and fails at runtime, in the middle of a command that writes backlog files. A test that decodes the produced payload into `ReviewTargets` and asserts `skill` / `targets[i].name` / `targets[i].path` turns that class of drift into a compile-or-test failure. Same for the registration key: it is a string constant shared across a crate boundary with nothing checking the two ends agree.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test decodes the payload produced by provide() into ops_create_review_tasks::ReviewTargets and asserts skill plus every target's name and path
- [ ] #2 A test asserts RustReviewTargetsProvider::name() equals ops_create_review_tasks::DATA_PROVIDER_NAME
- [ ] #3 A test registers CreateReviewTasksRustExtension's data providers into a registry and asserts the review_targets key resolves to this provider
- [ ] #4 The cfg_attr(test, allow(...)) block in lib.rs is either justified by the new tests or removed
<!-- AC:END -->
