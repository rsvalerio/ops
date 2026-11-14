---
id: TASK-2187
title: 'API-14: ops-about-go''s only public type has no doc summary, and crate-internal helpers are pub instead of pub(crate)'
status: Done
assignee: []
created_date: '2026-09-08 07:14'
updated_date: '2026-09-10 19:03'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2247'
modified_files:
  - extensions-go/about/src/lib.rs
  - extensions-go/about/src/go_syntax.rs
  - extensions-go/about/src/go_mod.rs
  - extensions-go/about/src/go_work.rs
  - extensions-go/about/src/modules.rs
priority: low
ordinal: 100000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/lib.rs:33`, `extensions-go/about/src/go_syntax.rs`, `extensions-go/about/src/go_mod.rs:26,29,37`, `extensions-go/about/src/go_work.rs:20`, `extensions-go/about/src/modules.rs:13,15,175`

**What**: Two related gaps in the crate's declared surface.

1. `#[non_exhaustive] pub struct AboutGoExtension;` (`lib.rs:33`) is the crate's sole exported item and carries no `///` doc summary at all — it is the one thing rustdoc renders for this crate. `GoIdentityProvider::provide` and `about_fields` are likewise undocumented (private, so lower stakes).

2. Everything in the four private modules is declared `pub` rather than `pub(crate)`, so the visibility marker no longer says anything about intent:
   - `go_syntax.rs`: `strip_line_comment`, `is_block_opener`, `is_block_terminator`, `strip_verb`, `unquote_token`, `has_embedded_parent_dir_segment`
   - `go_mod.rs`: `pub struct GoMod` (whose fields are `pub(crate)` — the struct is more visible than its own fields), `pub fn parse`
   - `go_work.rs`: `pub fn parse_use_dirs`
   - `modules.rs`: `pub const PROVIDER_NAME`, `pub struct GoUnitsProvider`, `pub fn last_segment`

   `mod go_syntax;` etc. are private, so none of these are reachable externally; the `pub` is inert and misleading. `GoMod`'s split visibility is the clearest symptom.

**Why it matters**: A reader cannot tell which items are the crate's contract and which are internal plumbing, and the one item that *is* the contract is the only one with no explanation of what it does. This is the Go-stack twin of TASK-2163 (ops-about-rust) and TASK-2071 (ops-about); the same policy should be applied to all three so the about extensions read consistently.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 AboutGoExtension carries a doc summary describing what the extension registers
- [x] #2 Every item in go_syntax, go_mod, go_work and modules that is not reachable outside the crate is pub(crate), and GoMod's visibility matches its fields'
- [x] #3 cargo doc and clippy pass with no new warnings after the visibility change
- [x] #4 The convention matches whatever TASK-2163 and TASK-2071 settle on for the Rust and generic about crates

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC2 substituted: pub(crate) on items in the private go_* modules is denied by clippy::redundant_pub_crate (workspace convention spells such items pub; see ops-cargo-toml/src/lib.rs). External reachability is already nil via the private modules; AC1/AC4 doc work done, wording consistent with TASK-2163/2071.

AC #2 audit follow-up: the pub->pub(crate) half is substituted (every module in extensions-go/about/src is private, so `pub` inside them is already crate-internal and `pub(crate)` there is what clippy::nursery/redundant_pub_crate denies workspace-wide). Two real gaps found in the post-fix audit were closed in this wave: GoMod's fields were pub(crate) while the struct was pub -- the fields are now pub and documented, matching the type and the ops-about-node convention; and a crate-level note recording the visibility decision was added above the module block in extensions-go/about/src/lib.rs.

<!-- SECTION:NOTES:END -->
