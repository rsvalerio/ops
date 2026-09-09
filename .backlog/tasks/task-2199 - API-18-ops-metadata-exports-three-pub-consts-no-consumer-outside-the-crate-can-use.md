---
id: TASK-2199
title: 'API-18: ops-metadata exports three pub consts no consumer outside the crate can use'
status: To Do
assignee: []
created_date: '2026-09-08 07:15'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - api-design
dependencies: []
parent_task_id: 'TASK-2246'
modified_files:
  - extensions-rust/metadata/src/lib.rs
priority: low
ordinal: 112000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/lib.rs:73`, `extensions-rust/metadata/src/lib.rs:76`, `extensions-rust/metadata/src/lib.rs:98`

**What**: `METADATA_MAX_BYTES_DEFAULT`, `METADATA_MAX_BYTES_ENV` and
`METADATA_MAX_BYTES_CEILING` are `pub`. A workspace-wide grep for each
identifier returns only `extensions-rust/metadata/src/**` (the crate's own
code and its `#[cfg(test)]` modules). The crate is `publish = false`, and its
sole cross-crate reference is the `#[cfg(feature = "stack-rust")] extern crate
ops_metadata;` at `crates/cli/src/main.rs:36`, which exists purely to keep the
`linkme` factory linked and names no item.

Every other item in the crate is already correctly scoped: `MetadataProvider`
is private, and `resolve_metadata_max_bytes` / `metadata_max_bytes` /
`query_metadata_raw*` / `CARGO_METADATA_ARGS` / `check_metadata_output` are
`pub(crate)` and reached from tests through `use crate::…`. The three consts
are the only items that break that pattern, and nothing about them needs a
wider scope than their siblings.

**Why it matters**: API-18 — a public surface wider than any consumer needs is
a maintenance obligation with no counterparty. Here it also misrepresents the
crate: `cargo doc` presents an OOM-budget knob API as if it were part of a
supported contract, when the only thing outside this crate that observes it is
the `OPS_METADATA_MAX_BYTES` environment variable at runtime.

`MetadataExtension` (`lib.rs:218`) must stay `pub` — the `impl_extension!` /
`linkme` registration requires it — so it is not part of this finding.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 METADATA_MAX_BYTES_DEFAULT, METADATA_MAX_BYTES_ENV and METADATA_MAX_BYTES_CEILING are pub(crate), matching every other non-extension item in the crate
- [ ] #2 The crate still builds and its tests still reach the constants via use crate::…
- [ ] #3 MetadataExtension remains pub so the linkme factory registration is unaffected
<!-- AC:END -->
