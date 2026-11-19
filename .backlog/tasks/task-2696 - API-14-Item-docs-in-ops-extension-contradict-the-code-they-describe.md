---
id: TASK-2696
title: 'API-14: Item docs in ops-extension contradict the code they describe'
status: Done
assignee: []
created_date: '2026-11-18 18:29'
updated_date: '2026-11-18 22:11'
labels:
  - code-review-rust
  - API
dependencies: []
parent_task_id: 'TASK-2844'
modified_files:
  - crates/extension/src/data.rs
  - crates/extension/src/extension.rs
priority: low
ordinal: 1000
dedup_key: 'API-14:crates/extension/src/data.rs:DataRegistry::schemas'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs:336`, `crates/extension/src/data.rs:106`, `crates/extension/src/extension.rs:412-413`

**What**: Three public docs describe behaviour the code does not have:
- `DataRegistry::schemas` (data.rs:336) says it "Returns schemas for all providers that have non-empty descriptions", but the body returns every provider's schema unfiltered, and `tests/public_api.rs::data_registry_schemas_returns_sorted` pins the unfiltered behaviour (a `StubProvider` with an empty default schema is counted).
- The `DataProvider` trait example (data.rs:106) says "possibly using ctx.db", but `Context::db` is a private field; the accessor is `ctx.db()` and only exists under the `sqlite` feature.
- `Extension::register_commands` (extension.rs:412-413) describes last-write-wins command registration and then says "see [`DataRegistry::register`] for the full contract", which is the *data-provider* (first-write-wins) registry; the matching contract is on `CommandRegistry::insert`.

**Why it matters**: API-14: the doc summary is the contract readers act on. A caller relying on `schemas()` to skip undescribed providers will list them anyway, and the wrong cross-reference points readers at the opposite duplicate policy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 DataRegistry::schemas docs match its behaviour (or the behaviour is changed to match and the test updated)
- [x] #2 DataProvider example refers to ctx.db() and notes the sqlite feature gate
- [x] #3 Extension::register_commands links to CommandRegistry::insert for its duplicate contract

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.04 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
