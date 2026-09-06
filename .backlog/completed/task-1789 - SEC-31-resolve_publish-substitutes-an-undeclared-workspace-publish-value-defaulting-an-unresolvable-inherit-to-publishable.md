---
id: TASK-1789
title: >-
  SEC-31: resolve_publish substitutes an undeclared workspace publish value,
  defaulting an unresolvable inherit to publishable
status: Done
assignee:
  - TASK-1994
created_date: '2026-08-27 11:23'
updated_date: '2026-08-28 20:18'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions-rust/cargo-toml/src/inheritance.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/inheritance.rs:140-144` (`resolve_publish`), interacting with `extensions-rust/cargo-toml/src/types.rs:257` (`PublishSpec::is_publishable`).

**What**: every other resolver in this module refuses to substitute when the workspace side did not declare the field:

- `resolve_string_field` (`:99`) requires `ws_value` to be `Some`.
- `resolve_vec_field` (`:116`) additionally requires the workspace vec to be non-empty (TASK-0961).
- `resolve_readme` (`:131`) requires `ws_value` to be `Some`.
- `resolve_optional_string` (`:124`) delegates to `resolve_string_field`.

`resolve_publish` alone substitutes unconditionally:

```rust
if matches!(field, PublishSpec::Inherited { workspace: true }) {
    *field = ws_value.clone();
}
```

`WorkspacePackage::publish` is `#[serde(default)]` over `PublishSpec`, whose `#[default]` is `PublishSpec::None`. So when a manifest says `publish = { workspace = true }` and the `[workspace.package]` table has **no** `publish` key, the member's field is rewritten from `Inherited` to `None`, and `is_publishable()` then returns `Some(true)` — "publishable to any registry".

**Why it matters**: this is the exact failure mode TASK-1196 changed `is_publishable` from `bool` to `Option<bool>` to prevent. That fix made the *unresolved* shape surface as `None` so a caller gating `cargo publish` must handle it explicitly. `resolve_package_inheritance` then erases the signal: after resolution there is no longer any way to distinguish "the workspace really said publish is allowed" from "the workspace never declared publish, and cargo would have hard-errored on this manifest". The safe default silently flips to open. Fail-closed (SEC-31) requires the unresolvable case to stay unresolved, exactly as the sibling resolvers do for version/license/readme/keywords.

Note the asymmetry is invisible in the current suite: `resolve_package_inheritance_keywords_categories_readme_license_file_and_publish` (`src/tests/inheritance.rs:118`) only exercises the case where the workspace *does* declare `publish = false`, and `resolve_package_inheritance_missing_ws_value_stays_inherited` (`:502`) covers the missing-value case for `version` only.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 resolve_publish leaves the field as PublishSpec::Inherited when the workspace value is PublishSpec::None (i.e. the workspace never declared publish), matching resolve_string_field/resolve_vec_field/resolve_readme
- [x] #2 After resolve_package_inheritance on a manifest with 'publish = { workspace = true }' and a [workspace.package] table lacking a publish key, is_publishable() returns None, not Some(true)
- [x] #3 The existing behaviour is preserved when the workspace does declare publish (Bool or Registries), including publish = false
- [x] #4 A test in src/tests/inheritance.rs covers the undeclared-workspace-publish case, mirroring resolve_package_inheritance_missing_ws_value_stays_inherited
- [x] #5 The doc comment on resolve_publish states the fail-closed rule
<!-- AC:END -->
