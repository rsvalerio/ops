---
id: TASK-0031
title: "extension/src/lib.rs is a 1069-line god module with 6+ distinct concerns"
status: Triage
assignee: []
created_date: '2026-04-09 20:35:00'
labels: [rust-code-quality, CQ, ARCH-1, medium, crate-extension]
dependencies: [TASK-0018]
---

## Description

**Location**: `crates/extension/src/lib.rs:1-798`
**Anchor**: `mod extension` (entire module)
**Impact**: The file mixes at least 6 distinct concerns in ~600 lines of non-test code (plus ~270 lines of tests), making it hard to navigate and increasing cognitive load when modifying any single concern.

**Notes**:
The module conflates these responsibilities:

1. **Error wrappers** (lines 20-51): `SharedError`, `From<anyhow::Error>`, `From<serde_json::Error>`
2. **Type definitions** (lines 54-106): `ExtensionType` bitflags, `ExtensionInfo`, `DataField`, `DataProviderSchema`
3. **Data provider trait + error** (lines 108-249): `DataProviderError`, `DataProvider` trait, `DataRegistry`
4. **Context** (lines 257-305): `Context` struct with caching logic
5. **Extension trait** (lines 307-380): `Extension` trait with default methods
6. **Macros** (lines 406-571): `impl_extension!` (4 variants), `data_field!`, `test_datasource_extension!`

ARCH-1 flags modules >500 lines with mixed unrelated concerns. Suggested split:

- `error.rs` — `SharedError`, `DataProviderError`, `From` impls
- `data.rs` — `DataField`, `DataProviderSchema`, `DataProvider`, `DataRegistry`
- `context.rs` — `Context`
- `traits.rs` — `Extension`, `ExtensionInfo`, `ExtensionType`
- `macros.rs` — all three macros
- `lib.rs` — thin re-exports, `ExtensionFactory`, `EXTENSION_REGISTRY`

TASK-0018 addresses the `impl_extension!` macro's cognitive load specifically; this task covers the broader module structure.
