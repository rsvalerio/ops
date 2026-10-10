---
id: TASK-2515
title: 'API-13: ops-rust-foundation Drift and Rule are reachable by two public paths (pub mod compare plus root pub use)'
status: To Do
assignee: []
created_date: '2026-10-10 15:32'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2616'
modified_files:
  - extensions-rust/foundation/src/lib.rs
priority: medium
ordinal: 1000
dedup_key: 'API-13:extensions-rust/foundation/src/lib.rs:lib'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/foundation/src/lib.rs:16` and `extensions-rust/foundation/src/lib.rs:27`

**What**: `pub mod compare;` (lib.rs:16) keeps the module public while `pub use compare::{Drift, Rule};` (lib.rs:27) re-exports the same types at the root, so `ops_rust_foundation::Drift` and `ops_rust_foundation::compare::Drift` are both live public paths for one type — the characteristic `pub use` that duplicates a still-public path (API-13). The only external consumer, `crates/cli/src/init_rust_cmd.rs`, uses root paths exclusively (`ops_rust_foundation::{Outcome, Report}`, `::Drift`, `::scaffold`, `::check`); nothing outside the crate uses `ops_rust_foundation::compare::*`.

**Why it matters**: Every mention of `Drift`/`Rule` in docs, search results and error messages doubles, and readers are left unsure whether the two paths are the same type. Re-exporting from a private module to lift items into the root is the *correct* use of `pub use` — the item then has exactly one public path.

**Fix note**: making the module private also requires rewording the intra-doc link in the crate docs (`The check is semantic (see `[`compare`]`)`, lib.rs:11), because `broken_intra_doc_links` is denied at workspace level and a public doc cannot link to a private module.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 compare module is private (mod compare;) with the root pub use kept, so Drift and Rule have exactly one public path
- [ ] #2 The intra-doc link to compare in the crate docs is reworded or inlined so rustdoc builds with broken_intra_doc_links = deny
- [ ] #3 cargo check -p ops and cargo test -p ops-rust-foundation pass
<!-- AC:END -->
