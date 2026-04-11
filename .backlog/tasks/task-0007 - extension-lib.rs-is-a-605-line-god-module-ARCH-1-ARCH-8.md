---
id: TASK-0007
title: 'extension/lib.rs is a 605-line god module (ARCH-1, ARCH-8)'
status: Done
assignee: []
created_date: '2026-04-10 16:00:00'
updated_date: '2026-04-11 10:09'
labels:
  - rust-code-quality
  - CQ
  - ARCH-1
  - ARCH-8
  - medium
  - crate-extension
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/extension/src/lib.rs:1-605`
**Anchor**: `lib.rs` (entire file)
**Impact**: The entire extension crate lives in a single `lib.rs` — 605 lines of production code mixing error types (`SharedError`, `DataProviderError`), the data provider system (`DataProvider` trait, `DataRegistry`, `Context`, `DuckDbHandle`), the extension trait system (`Extension`, `ExtensionInfo`, `ExtensionType`), and three macros (`impl_extension`, `data_field`, `test_datasource_extension`). Per ARCH-8, `lib.rs` should be a thin entry point with module declarations and re-exports.

**Notes**:
Suggested split:
- `lib.rs` — thin entry point: module declarations, crate-level docs, re-exports
- `error.rs` — `SharedError`, `DataProviderError` (45 lines + impls)
- `data.rs` — `DataProvider` trait, `DataRegistry`, `Context`, `DuckDbHandle`, `DataField`, `DataProviderSchema` (~150 lines)
- `extension.rs` — `Extension` trait, `ExtensionInfo`, `ExtensionType`, `CommandRegistry`, `ExtensionFactory`, `EXTENSION_REGISTRY` (~120 lines)
- `macros.rs` — `impl_extension!`, `data_field!`, `test_datasource_extension!` (~170 lines)

The crate is cohesive (all extension framework code), so the concern is navigation and change isolation rather than coupling. The split would make it easier to find and modify one subsystem without scrolling past the others.

ARCH-1: module >500 lines. ARCH-8: lib.rs should be thin entry point with module declarations and re-exports, not implementation.
<!-- SECTION:DESCRIPTION:END -->
