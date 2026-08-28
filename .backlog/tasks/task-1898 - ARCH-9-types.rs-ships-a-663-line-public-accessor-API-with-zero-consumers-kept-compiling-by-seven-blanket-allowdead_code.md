---
id: TASK-1898
title: >-
  ARCH-9: types.rs ships a 663-line public accessor API with zero consumers,
  kept compiling by seven blanket allow(dead_code)
status: To Do
assignee:
  - TASK-1999
created_date: '2026-08-27 15:36'
updated_date: '2026-08-28 14:14'
labels:
  - code-review-rust
  - structure-readability
dependencies: []
modified_files:
  - extensions-rust/metadata/src/types.rs
  - extensions-rust/metadata/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/types.rs` (whole file; `#[allow(dead_code)]` at :150, :182, :373, :381, :504, :516, :523, :587, :594)

**What**: `Metadata`, `Package`, `Dependency`, `Target` and their ~50 accessors are re-exported from `lib.rs:22` and used by nothing in the workspace. The only reference to the crate outside its own directory is `extern crate ops_metadata;` in `crates/cli/src/main.rs:36`, present solely so the `linkme` extension factory registers — it pulls in no items. `Metadata::from_context`, the doc-comment's designated production entry point ("about, deps, units, coverage providers share one allocation"), has zero callers and zero tests.

Everything that reaches production goes through `MetadataProvider::provide` -> `provide_from_db` -> `query_metadata_raw`, which returns a raw `serde_json::Value` and never constructs a `Metadata`. The typed layer is parallel to the shipped path, not on it.

The nine `#[allow(dead_code)]` attributes are the tell. They are also, for the most part, inert: `dead_code` does not fire on `pub` items reachable from the crate root, which these are. So they suppress nothing today while guaranteeing that genuinely dead private helpers added inside those impls in future are never reported — `json_str_with_fallback` and `json_bool_with_fallback` (:93, :101) are already `pub` in a private module, i.e. unreachable from outside the crate.

The doc comments here are unusually careful (cache-lifetime reasoning at :112-133, duplicate-id first-write-wins policy at :239-275, the READ-1 note at :5-14 correcting an earlier comment that cited macros which never existed). That care has gone into an API no caller exercises, and the 30-odd accessor tests in `tests/accessors.rs` are testing `serde_json::Value::get` wrappers rather than any behaviour the binary depends on.

**Why it matters**: ARCH-9 (minimal public surface). Either this layer has a planned consumer, in which case the plan belongs in the module doc and `from_context` needs a test (nothing currently proves the `DataProviderError` mapping or the `Arc` sharing it documents), or it does not, in which case 663 lines of source plus ~600 lines of tests are maintenance surface with no user. The blanket `allow(dead_code)` is what lets the question stay unanswered indefinitely.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A decision is recorded: either the typed accessor layer has a named consumer (documented in the module header) or the unconsumed surface is removed
- [ ] #2 If kept, every #[allow(dead_code)] is removed and the crate still compiles clean under -D warnings; any attribute that turns out to be load-bearing is narrowed to the specific item and carries a reason comment per docs/clippy.md
- [ ] #3 If kept, json_str_with_fallback and json_bool_with_fallback are made pub(crate) — they are pub in a private module and unreachable as written
<!-- AC:END -->
