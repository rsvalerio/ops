---
id: TASK-1814
title: >-
  DUP-3: member_package_name re-implements ops-about-rust's crate-manifest
  reader and drops its SEC-14 member-path guard
status: Triage
assignee: []
created_date: '2026-08-27 11:32'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions-rust/create-review-tasks/src/provider.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/create-review-tasks/src/provider.rs:29-37`, `extensions-rust/create-review-tasks/src/provider.rs:57-81`

**What**: `member_package_name` is a near-verbatim copy of `ops_about_rust::units::read_crate_metadata` (`extensions-rust/about/src/units.rs:188-232`): same `ops_core::text::read_capped_to_string` call, same silent-on-`NotFound` / `debug`-on-other-read-error / `warn`-on-parse-error ladder, same `CargoToml::parse` + `package_name().map(str::to_string)` tail. The call site at provider.rs:29-37 (`root.join(member).join("Cargo.toml")` → package name → `format_unit_name(member)` fallback) is the same function as `ops_about_rust::units::resolve_crate_display_name` (`extensions-rust/about/src/units.rs:243-255`), down to the identical fallback helper. The doc comment on `member_package_name` even says it "follows the read-log policy of the about units provider" — the policy is being restated in a second copy instead of called.

The copy is not faithful in one respect: `resolve_crate_display_name` opens with a `member_path_is_workspace_safe(member)` guard, added for SEC-14 / TASK-1246 precisely because `Path::join` discards the root when `member` is absolute and walks parents on `..`, so an unchecked join can drive `read_capped_to_string` and tracing breadcrumbs at an arbitrary filesystem location. provider.rs:30 performs the same join with no guard. Today the input happens to be pre-filtered — `resolved_workspace_members` drops unsafe members at `extensions-rust/about/src/query.rs:522-528` — so this is not currently exploitable; it is the defence-in-depth layer the about crate deliberately keeps at both levels, and it silently disappears the moment this provider is fed a member list from anywhere else.

**Cross-crate cause**: `ops_about_rust::units` is `pub(crate)`; the crate's `lib.rs` re-exports only `resolved_workspace_members`. Reusing the shared helper therefore requires `ops-about-rust` to re-export `resolve_crate_display_name` (and/or `read_crate_metadata`) the same way it already re-exports `resolved_workspace_members` for "sibling Rust-stack extension crates (about, create-review-tasks-rust)" — see the comment at `extensions-rust/about/src/lib.rs:37-41`. This crate already depends on `ops-about-rust`.

**Why it matters**: two copies of the manifest read/parse/log policy drift independently — a fix to the read cap, the log levels, or the path guard in one is invisible to the other, and the SEC-14 divergence is already an instance of exactly that drift. Reusing the existing helper deletes ~25 lines here and restores the guard for free.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 member_package_name is removed and the display-name resolution delegates to the shared ops-about-rust helper (re-exported from that crate's lib.rs alongside resolved_workspace_members)
- [ ] #2 The member -> Cargo.toml join is guarded against absolute and '..' member entries, matching resolve_crate_display_name's SEC-14 / TASK-1246 behaviour
- [ ] #3 Existing tests (package names, missing manifest fallback, malformed manifest fallback) still pass unchanged against the shared helper
- [ ] #4 A test covers an absolute or '..' member entry reaching the provider and asserts no read is attempted outside the workspace root
<!-- AC:END -->
