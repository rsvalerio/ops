---
id: TASK-1740
title: 'API-9: #[non_exhaustive] on PackageJson is a no-op — the type is crate-private'
status: Done
assignee:
  - TASK-1991
created_date: '2026-08-27 11:13'
updated_date: '2026-08-28 14:49'
labels:
  - code-review-rust
  - api
dependencies: []
modified_files:
  - extensions-node/about/src/package_json.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/package_json.rs:9-21`

**What**: `PackageJson` carries `#[non_exhaustive]`, but the type is unreachable from outside the crate: `mod package_json;` is declared without `pub` in `lib.rs:21`, and every field is `pub(crate)`. `#[non_exhaustive]` has no effect within the crate that defines the type — struct literals, exhaustive destructuring, and `..` patterns all still work, and `lib.rs:73-83` does in fact destructure the struct exhaustively. So the attribute constrains nobody and enforces nothing.

**Why it matters**: `#[non_exhaustive]` is a public-API commitment (API-9). Placing it on a crate-private type states an intent the compiler will not honour, and the contrast with `AboutNodeExtension` (`lib.rs:38-39`) — where the attribute *is* load-bearing, since that type is genuinely public and constructed by consumers — makes the crate-private use read as meaningful when it is not. A reader auditing the crate's public surface has to check the module visibility to discover the attribute is inert, and a future move of this type into the public surface would silently inherit a guarantee that was never tested.

Same observation applies to the `pub struct` / `pub(crate)` field mix: the struct's own `pub` is already downgraded to crate visibility by the private module, so the two visibility levels on the type and its fields are describing the same thing twice.

**Fix shape**: drop `#[non_exhaustive]` from `PackageJson` and let `pub(crate) struct PackageJson` with `pub(crate)` fields state the actual visibility. Keep the attribute on `AboutNodeExtension`, where it is real.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 #[non_exhaustive] is removed from PackageJson, or the type is made genuinely public so the attribute is load-bearing
- [x] #2 PackageJson's declared visibility matches its reachable visibility (no pub struct behind a private module)
- [x] #3 #[non_exhaustive] remains on AboutNodeExtension, which is genuinely part of the public surface
- [x] #4 cargo clippy -p ops-about-node and cargo test -p ops-about-node pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented in wave TASK-1991: dropped #[non_exhaustive]; type and fields now uniformly `pub` behind the private `mod package_json` (clippy::redundant_pub_crate, a denied nursery lint, rejects `pub(crate)` inside a private module, so uniform `pub` — not `pub(crate)` — is the clippy-clean way to state one visibility once).
<!-- SECTION:NOTES:END -->
