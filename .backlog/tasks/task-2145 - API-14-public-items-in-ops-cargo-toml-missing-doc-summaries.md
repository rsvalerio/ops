---
id: TASK-2145
title: 'API-14: public items in ops-cargo-toml missing doc summaries'
status: To Do
assignee: []
created_date: '2026-09-08 07:02'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2247'
modified_files:
  - extensions-rust/cargo-toml/src/lib.rs
  - extensions-rust/cargo-toml/src/workspace_root.rs
  - extensions-rust/cargo-toml/src/inheritance.rs
  - extensions-rust/cargo-toml/src/types.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/lib.rs:108`

**What**: Public items exported from `ops-cargo-toml` with no doc summary:

- `extensions-rust/cargo-toml/src/lib.rs:108` — `pub const NAME`
- `extensions-rust/cargo-toml/src/lib.rs:109` — `pub const DESCRIPTION`
- `extensions-rust/cargo-toml/src/lib.rs:110` — `pub const SHORTNAME`
- `extensions-rust/cargo-toml/src/lib.rs:111` — `pub const DATA_PROVIDER_NAME` (the registry key other extensions look the provider up by — `extensions-rust/about/src/manifest.rs:287` reads `ops_cargo_toml::DATA_PROVIDER_NAME`)
- `extensions-rust/cargo-toml/src/workspace_root.rs:23` — `FindWorkspaceRootError::NotFound` variant and its `start` / `depth` fields
- `extensions-rust/cargo-toml/src/workspace_root.rs:25` — `FindWorkspaceRootError::CanonicalizeFailed` variant and its `path` / `source` fields
- `extensions-rust/cargo-toml/src/inheritance.rs:20` — `InheritanceError::MissingWorkspaceDependency`'s `name` / `section` fields (the variant itself is documented)
- `extensions-rust/cargo-toml/src/types.rs:14` — `ParseError`'s wrapped `toml::de::Error` field

The workspace does not enable `missing_docs` (root `Cargo.toml` `[workspace.lints.rust]`), so nothing catches these.

**Why it matters**: The four constants are the crate's public identity — the extension name, shortname and the data-provider registry key consumers hard-depend on — and rustdoc renders them with no explanation of which is which or that `DATA_PROVIDER_NAME` differs from `NAME` by underscore-vs-hyphen. The undocumented error variants and fields are the surface consumers match on (`is_not_found` exists because `about` matches on this type).

Sibling per-crate findings: TASK-2071 (ops-about), TASK-2097/2098 (ops-core, ops-extension), TASK-2110 (ops-git), TASK-2124, TASK-2130, TASK-2138, TASK-2139.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every public const in ops-cargo-toml carries a one-line doc summary saying what it names and where it is consumed
- [ ] #2 Every public enum variant and public field of FindWorkspaceRootError, InheritanceError and ParseError carries a doc summary
- [ ] #3 cargo doc -p ops-cargo-toml renders no undocumented public item
<!-- AC:END -->
