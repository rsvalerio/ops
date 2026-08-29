---
id: TASK-1793
title: >-
  ERR-6: InheritableField::default() uses an empty-string sentinel, making an
  absent [package] version indistinguishable from version = ""
status: Done
assignee:
  - TASK-1994
created_date: '2026-08-27 11:24'
updated_date: '2026-08-28 20:19'
labels:
  - code-review-rust
  - api-design
dependencies: []
modified_files:
  - extensions-rust/cargo-toml/src/types.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/types.rs:193-197` (`impl<T: Default> Default for InheritableField<T>`), consumed via `types.rs:107-155` (`Package::version`, `edition`, `rust_version`, `description`, `documentation`, `homepage`, `repository`, `license`, `authors`, `keywords`, `categories` — all `#[serde(default)]`) and `types.rs:88` (`CargoToml::package_version`).

**What**: the `Default` impl produces `InheritableField::Value(T::default())`, i.e. `Value("")` for every `InheritableString`. A `[package]` table with no `version` key therefore parses to `Value("")`, and:

- `package_version()` returns `Some("")` — a sentinel, not an absence. Pinned as intended by `src/tests/parse_edge.rs:49-57` and `src/tests/inheritance.rs:351-357`.
- `Package::version.as_str()` returns `Some("")`, which is truthy for every `Option`-based fallback chain.

Cargo's own semantics differ on both counts: an omitted `version` means `0.0.0`, and an omitted `description`/`license` means *absent*, not empty.

**Cross-crate consequence** (cause is here, in this crate's type design): `extensions-rust/about/src/identity/resolver.rs:23-26` resolves each identity field as

```rust
pkg.and_then(&pkg_getter).or_else(|| ws_pkg.and_then(&ws_getter))
```

with `pkg_getter = |p| p.$field.as_str()`. Because `as_str()` yields `Some("")` rather than `None` for an unset field, the `[workspace.package]` fallback **never fires** for any field the member omitted entirely — `about` reports an empty version / description / license instead of the workspace's value. Note the `authors` arm immediately below it (`resolver.rs:41-49`) had to hand-roll a `.filter(|wp| !wp.authors.is_empty())` guard for the same reason; the string fields have no such guard.

Contrast the sibling case that was already fixed: `resolve_vec_field` (`inheritance.rs:116`, TASK-0961) explicitly treats an empty workspace vec as "not declared" precisely because `Vec` has the same absent/empty collision.

**Why it matters**: ERR-6 — a sentinel value stands in for a state the type could represent directly. Every consumer must remember to `.filter(|s| !s.is_empty())`, and the one consumer in this repo does not.

Two fix shapes, either acceptable: (a) drop `#[serde(default)]` and make the fields `Option<InheritableString>` so absence is `None`; or (b) add an `InheritableField::Absent` variant (or make the accessors return `None` for an empty `Value`) so `as_str()` distinguishes the states. Option (a) is a breaking change to `Package`, which is `#[non_exhaustive]` and unpublished (`publish = false`), so the blast radius is in-repo only.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A [package] table with no version key is distinguishable from 'version = ""' through the public API (package_version() / Package::version)
- [x] #2 The same distinction holds for the other #[serde(default)] InheritableString fields: edition, rust_version, description, documentation, homepage, repository, license
- [x] #3 extensions-rust/about identity resolution falls back to [workspace.package] for a field the member omits entirely; a regression test covers version and description
- [x] #4 src/tests/parse_edge.rs:49 (parse_with_missing_required_version) and src/tests/inheritance.rs:351 (inheritable_field_default) are updated to pin the new contract rather than the empty-string sentinel
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Landed in wave TASK-1994. Fix shape (b) from the description: added an
`InheritableField::Absent` variant and made it the `Default`, so `#[serde(default)]`
absence is a state rather than `Value(T::default())`. Chosen over shape (a)
(`Option<InheritableString>`) because it leaves every call site's `.as_str()` /
`.value()` unchanged while still making absence observable.

- `Absent` is declared last so untagged deserialisation only reaches it after `Value`
  and `Inherited` are ruled out; it serialises as `null` and deserialises back, so the
  provider's typed -> JSON -> typed round-trip preserves the distinction
  (`inheritable_field_absent_round_trips_through_json`).
- AC #3: `about`'s identity resolver now reaches its `[workspace.package]` fallback for
  a field the member omits entirely — covered by
  `resolve_field_falls_back_to_workspace_when_package_omits_the_key` (version and
  description) with `resolve_field_declared_empty_string_beats_workspace` as the guard
  rail. No production change was needed in `about`: the sentinel was the whole bug.
- AC #4: `parse_with_missing_required_version` and `inheritable_field_default` (renamed
  `inheritable_field_default_is_absent`) now pin the new contract, plus
  `parse_with_empty_version_is_not_absent` and
  `parse_absent_inheritable_string_fields_are_all_absent` for AC #2's field sweep.
<!-- SECTION:NOTES:END -->
