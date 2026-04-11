---
id: TASK-004
title: "extension/src/lib.rs exceeds module size threshold with mixed concerns"
status: To Do
assignee: []
created_date: '2026-04-07 00:00:00'
labels: [rust-code-quality, CQ, ARCH-1, ARCH-8, medium, effort-M, crate-extension]
dependencies: []
---

## Description

**Location**: `crates/extension/src/lib.rs:1-1069`
**Anchor**: `mod` (entire file)
**Impact**: At 1,069 lines (~600 non-test), this single file mixes several unrelated concerns: error types (`SharedError`, `DataProviderError`), bitflags (`ExtensionType`), data provider trait + registry (`DataProvider`, `DataRegistry`), execution context (`Context`), extension trait (`Extension`), extension info (`ExtensionInfo`), and 4 macro variants (`impl_extension!`, `data_field!`, `test_datasource_extension!`). ARCH-1 flags >500 lines with mixed concerns; ARCH-8 says lib.rs should be a thin entry point.

**Notes**:
Suggested split:
- `error.rs` — `SharedError`, `DataProviderError`, `From` impls (~50 lines)
- `data.rs` — `DataProvider` trait, `DataField`, `DataProviderSchema`, `DataRegistry` (~100 lines)
- `context.rs` — `Context` struct and methods (~50 lines)
- `extension.rs` — `Extension` trait, `ExtensionInfo`, `ExtensionType` bitflags (~80 lines)
- `macros.rs` — `impl_extension!`, `data_field!`, `test_datasource_extension!` (~100 lines)
- `lib.rs` — module declarations, re-exports, `ExtensionFactory`, `EXTENSION_REGISTRY` (~30 lines)

The file is well-documented and the code is clean, but the single-file layout increases cognitive load when navigating. The types have distinct responsibilities and natural module boundaries.
