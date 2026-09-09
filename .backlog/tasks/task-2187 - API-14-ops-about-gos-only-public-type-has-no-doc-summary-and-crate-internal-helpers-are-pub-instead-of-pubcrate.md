---
id: TASK-2187
title: 'API-14: ops-about-go''s only public type has no doc summary, and crate-internal helpers are pub instead of pub(crate)'
status: To Do
assignee: []
created_date: '2026-09-08 07:14'
updated_date: '2026-09-08 20:00'
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
- [ ] #1 AboutGoExtension carries a doc summary describing what the extension registers
- [ ] #2 Every item in go_syntax, go_mod, go_work and modules that is not reachable outside the crate is pub(crate), and GoMod's visibility matches its fields'
- [ ] #3 cargo doc and clippy pass with no new warnings after the visibility change
- [ ] #4 The convention matches whatever TASK-2163 and TASK-2071 settle on for the Rust and generic about crates
<!-- AC:END -->
