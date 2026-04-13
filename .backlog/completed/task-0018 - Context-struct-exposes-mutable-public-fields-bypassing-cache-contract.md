---
id: TASK-0018
title: 'Context struct exposes mutable public fields, bypassing cache contract'
status: Done
assignee: []
created_date: '2026-04-10 23:30:00'
updated_date: '2026-04-11 09:55'
labels:
  - rust-code-quality
  - CQ
  - ARCH-9
  - API-9
  - medium
  - crate-extension
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/extension/src/lib.rs:257-265`
**Anchor**: `struct Context`
**Impact**: All five fields of `Context` are `pub`, including `data_cache: HashMap<String, Arc<serde_json::Value>>`. The `get_or_provide` method (line 292) implements a cache-or-compute contract: check cache → compute via registry → store in cache → return. Because `data_cache` is public, any code holding a `&mut Context` can insert stale entries, remove cached values, or clear the cache entirely, violating the invariant that cached values match their provider's output. Similarly, `config` and `working_directory` can be silently swapped mid-operation.

**Notes**:
ARCH-9: "Minimal public surface; hide internals behind modules." API-9: "Private unit field pattern" for structs that should control construction.

`Context` is passed to `DataProvider::provide(&self, ctx: &mut Context)` — providers legitimately need to read `config`, `working_directory`, and `refresh`, and use `db`. But they should not need direct write access to `data_cache`.

Fix options (from least to most disruptive):
1. **Accessor methods**: Make `data_cache` private, keep other fields `pub`. Add `pub fn cached(&self, key: &str) -> Option<&Arc<Value>>` if providers need to peek. This preserves the cache contract while keeping the struct easy to construct.
2. **Builder/constructor**: Make all fields private, require construction via `Context::new()` (already exists). Add getters for read access. This is cleaner but requires updating all provider implementations that read `ctx.config` directly.
3. **Split struct**: Separate `ContextView` (read-only config/cwd/refresh) from `ContextCache` (mutable cache). Providers receive `&ContextView` for reads and the cache is managed internally. Most disruptive but cleanest separation.

Option 1 is recommended — it fixes the primary concern (cache bypass) with minimal churn.
<!-- SECTION:DESCRIPTION:END -->
