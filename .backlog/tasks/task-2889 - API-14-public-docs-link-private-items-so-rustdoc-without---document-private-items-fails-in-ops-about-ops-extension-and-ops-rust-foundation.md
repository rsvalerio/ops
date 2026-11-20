---
id: TASK-2889
title: 'API-14: public docs link private items, so rustdoc without --document-private-items fails in ops-about, ops-extension and ops-rust-foundation'
status: Done
assignee: []
created_date: '2026-11-19 17:20'
updated_date: '2026-11-20 18:26'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2891'
modified_files:
  - extensions/about/src/cards.rs
  - extensions/about/src/machine/cfg_expr.rs
  - extensions/about/src/machine/layers.rs
  - extensions/about/src/manifest_cache.rs
  - extensions/about/src/manifest_io.rs
  - crates/extension/src/context.rs
  - extensions-rust/foundation/src/lib.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/cards.rs:180`, `extensions/about/src/machine/cfg_expr.rs:26`, `extensions/about/src/machine/layers.rs:24`, `extensions/about/src/manifest_cache.rs:32`, `extensions/about/src/manifest_io.rs:17`, `crates/extension/src/context.rs:401`, `extensions-rust/foundation/src/lib.rs:380`

**What**: `RUSTDOCFLAGS="-D warnings" cargo rustdoc -p <crate>` fails with `rustdoc::private_intra_doc_links` on these public items' docs (they link `CardLayoutConfig::*`, `MAX_CFG_DEPTH`, `push_with_includes`, `is_evictable`, `read_contained`, `Context::enter_provider/exit_provider`, `check_workspaces`). `ops verify` builds docs with `--document-private-items`, so the gate never sees it.

**Why it matters**: Published (public-only) docs render dead links, and the new per-crate `cargo rustdoc -- -D missing-docs` gates cannot be extended to these crates without first clearing them.

**Origin**: discovered during TASK-2883 while fixing TASK-2875 (the same warning in ops-about-rust members.rs was fixed in-wave).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 RUSTDOCFLAGS='-D warnings' cargo rustdoc passes for ops-about, ops-extension and ops-rust-foundation without --document-private-items

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Private intra-doc links in public docs turned into plain code spans; RUSTDOCFLAGS='-D warnings' cargo rustdoc passes for ops-about, ops-extension and ops-rust-foundation. Regression gate filed as TASK-2896.
<!-- SECTION:NOTES:END -->
