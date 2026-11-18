---
id: TASK-2741
title: 'ERR-6: authors inheritance overwrites an undeclared workspace authors with an empty Value'
status: Done
assignee: []
created_date: '2026-11-18 18:38'
updated_date: '2026-11-18 22:30'
labels:
  - code-review-rust
  - error-handling
dependencies: []
parent_task_id: 'TASK-2850'
modified_files:
  - extensions-rust/cargo-toml/src/inheritance.rs
priority: low
ordinal: 1000
dedup_key: 'ERR-6:extensions-rust/cargo-toml/src/inheritance.rs:resolve_package_inheritance'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/inheritance.rs:84` (`CargoToml::resolve_package_inheritance`)

**What**: `authors` is resolved inline instead of through `resolve_vec_field`:

```rust
if matches!(&pkg.authors, InheritableField::Inherited { workspace: true }) {
    pkg.authors = InheritableField::Value(ws_pkg.authors.clone());
}
```

`WorkspacePackage::authors` is a `#[serde(default)] Vec<String>`, so an absent `[workspace.package] authors` parses as `[]`. `resolve_vec_field` (used for `keywords` and `categories` two lines above) documents exactly this and leaves the member `Inherited` when the workspace value is empty; the inline `authors` branch does not, so it turns `authors.workspace = true` into `Value(vec![])`.

**Why it matters**: The unresolved state is lost for one field only: consumers see a declared-but-empty authors list rather than "inherited, unresolved", which is the signal loss the `Absent`/`Inherited` design on `InheritableField` exists to prevent. The comment above the block ("each line routes one inheritable field through its matching resolver") is also untrue for this field.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 authors is resolved via resolve_vec_field (or equivalent) so an empty/undeclared workspace authors leaves the member field Inherited
- [x] #2 A test covers authors.workspace = true with no [workspace.package] authors and asserts the field stays Inherited

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.12 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
