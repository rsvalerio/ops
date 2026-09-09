---
id: TASK-2211
title: 'DUP-1: units.rs tests re-implement the shared write_file helper the sibling module in the same crate uses'
status: To Do
assignee: []
created_date: '2026-09-08 07:20'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - duplication
dependencies: []
parent_task_id: 'TASK-2242'
modified_files:
  - extensions-python/about/src/units.rs
priority: low
ordinal: 122000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/units.rs:179`

**What**: The `units.rs` test module defines its own fixture writer:

```rust
fn write(path: &Path, content: &str) {
    if let Some(parent) = path.parent() { std::fs::create_dir_all(parent).unwrap(); }
    std::fs::write(path, content).unwrap();
}
```

`ops_about::test_support::write_file` already exists and is exactly what the `lib.rs` test module in the same crate uses (`identity_from_with_files`, `invalid_pyproject_falls_back_to_directory_name_and_warns`). The crate already depends on `ops-about` with the `test-support` feature.

**Why it matters**: Two spellings of the same fixture helper in one crate — one shared, one local. A hardening of the shared helper (parent creation semantics, permissions, error messages) silently upgrades only half of this crate's tests, and the local copy is one more thing to keep in step.

**Note**: the same test module also re-imports `format_unit_name` behaviours already pinned in `ops_about`; only the `write` helper is in scope here.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 units.rs tests use ops_about::test_support::write_file and the local write helper is removed
- [ ] #2 The whole test module still passes unchanged
<!-- AC:END -->
