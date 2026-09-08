---
id: TASK-2163
title: >-
  API-14: ops-about-rust public surface lacks doc summaries and leaks pub on
  crate-internal helpers
status: To Do
assignee:
  - TASK-2247
created_date: '2026-09-08 07:05'
updated_date: '2026-09-08 11:00'
labels:
  - code-review-rust
  - api
dependencies: []
modified_files:
  - extensions-rust/about/src/lib.rs
  - extensions-rust/about/src/units.rs
  - extensions-rust/about/src/members.rs
priority: low
ordinal: 76000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/lib.rs:41-45`, `extensions-rust/about/src/lib.rs:58`, `extensions-rust/about/src/units.rs:22-31`, `extensions-rust/about/src/units.rs:264`, `extensions-rust/about/src/members.rs:346`

**What**: Two related gaps in the crate's public surface.

*Missing doc summaries.* Every public item declared in `lib.rs` is undocumented: `pub const NAME`, `pub const DESCRIPTION`, `pub const SHORTNAME`, `pub const DATA_PROVIDER_NAME` (`:41-45`) and `pub struct AboutRustExtension` (`:58`). So is `pub struct CrateMetadata` and its three public fields — `name`, `version`, `description` (`units.rs:22-27`) — which is re-exported from `lib.rs:56` and is therefore part of the crate's API for the sibling Rust-stack extensions the re-export block exists to serve. (The re-exported *functions* — `resolved_workspace_members`, `member_path_is_workspace_safe`, `read_crate_metadata` — are all well documented; it is the consts and the data type that are bare.)

*Over-broad visibility.* `units::resolve_crate_display_name` (`:264`) and `members::expand_member_glob` (`:346`) are declared `pub` but their modules are `pub(crate)` and neither is re-exported from `lib.rs`, so the `pub` is inert — it reads as public API to anyone editing the file while actually reaching nothing outside the crate. Both have exactly one caller each inside the crate (`coverage_provider.rs:283` and `members.rs:82`).

Note: this is the same rule as TASK-2071 but a different crate — that task targets `extensions/about` (the generic `ops_about` crate); this one is `extensions-rust/about` (`ops-about-rust`).

**Why it matters**: The re-export block at `lib.rs:46-56` states the crate deliberately exposes a shared API for sibling Rust-stack extensions, so `cargo doc` for that surface is the contract those siblings read — and it currently renders the identity constants and the `CrateMetadata` payload with no text at all. The inert `pub` markers work the other way: they overstate the surface, and a reader deciding whether a signature change is breaking has to check the module's visibility and the re-export list to find out that it is not.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `NAME`, `DESCRIPTION`, `SHORTNAME`, `DATA_PROVIDER_NAME` and `AboutRustExtension` each carry a one-line doc summary saying what the value is used for
- [ ] #2 `CrateMetadata` and each of its three public fields carry a doc summary, including what `None` means (read or parse failure, per `read_crate_metadata`'s contract)
- [ ] #3 `resolve_crate_display_name` and `expand_member_glob` are narrowed to `pub(crate)`, or re-exported from `lib.rs` if they are genuinely part of the sibling-extension API
- [ ] #4 `cargo doc -p ops-about-rust` produces no item without a summary line, and the crate still builds with `[lints] workspace = true`
<!-- AC:END -->
