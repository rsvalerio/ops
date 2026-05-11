---
id: TASK-1300
title: >-
  ERR-5: set_theme indexes doc["output"]["theme"] without verifying [output] is
  a table; non-table value panics in theme select
status: To Do
assignee:
  - TASK-1303
created_date: '2026-05-11 16:36'
updated_date: '2026-05-11 16:48'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/theme_cmd.rs:196-201`

**What**: `set_theme` does:
```rust
if !doc.contains_key("output") {
    doc["output"] = toml_edit::Item::Table(toml_edit::Table::new());
}
doc["output"]["theme"] = toml_edit::value(theme_name);
```
When `.ops.toml` already has `output = "something"` (or any non-table value, e.g. `output = 42`), `contains_key("output")` returns true so the guard does not replace it, and the subsequent index assignment `doc["output"]["theme"] = ...` panics inside `toml_edit` because you cannot index a `Value` as a table.

**Why it matters**: Operator-facing CLI panic (`thread 'main' panicked at ...`) instead of a clean error message. The same anti-pattern is what TASK-1278 calls out structurally ("ensure top-level table pattern triplicated"); this one in `theme_cmd` is broken — it only checks presence, not type. A malformed/legacy `.ops.toml` from a coworker turns `ops theme select` into a panic. Same fix needed for `[about]` / `[commands]` callers if they have the same shape, but at minimum: replace contains_key check with `as_table_mut()` or `entry(...).or_insert_with(Table)` and bail with an anyhow error on type mismatch.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 set_theme bails with a clear anyhow error (not a panic) when [output] exists but is not a table
- [ ] #2 Regression test exercises an input where output = "classic" (string, not table) and asserts a non-panic Err result
- [ ] #3 doc["output"]["theme"] = ... is replaced with a type-checked write via as_table_mut() or entry().or_insert(Table::new())
<!-- AC:END -->
