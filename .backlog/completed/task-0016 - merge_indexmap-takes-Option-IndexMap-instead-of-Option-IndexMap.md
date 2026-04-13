---
id: TASK-0016
title: merge_indexmap takes &Option<IndexMap> instead of Option<&IndexMap>
status: Done
assignee: []
created_date: '2026-04-10 20:30:00'
updated_date: '2026-04-11 09:55'
labels:
  - rust-idioms
  - EFF
  - OWN-6
  - low
  - crate-core
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/core/src/config/merge.rs:13-22`
**Anchor**: `fn merge_indexmap`
**Impact**: `merge_indexmap` accepts `overlay: &Option<IndexMap<K, V>>` instead of the idiomatic `overlay: Option<&IndexMap<K, V>>`. The `&Option<T>` pattern forces callers to pass a reference to the entire `Option` container rather than an optional reference to the inner value. While functionally equivalent here (callers destructure `ConfigOverlay` fields which are `Option<IndexMap<...>>`), it deviates from OWN-6.

**Notes**:
OWN-6: "Prefer `Option<&T>` over `&Option<T>`." The `&Option<T>` form is a Clippy lint (`clippy::ref_option_ref`, allow-by-default) and is flagged by `clippy::pedantic`.

Fix: Change signature to `Option<&IndexMap<K, V>>` and update the 4 call sites in `merge_config` to use `.as_ref()`:
```rust
pub(super) fn merge_indexmap<K: Clone + Eq + std::hash::Hash, V: Clone>(
    base: &mut IndexMap<K, V>,
    overlay: Option<&IndexMap<K, V>>,
) {
    if let Some(items) = overlay {
        for (k, v) in items {
            base.insert(k.clone(), v.clone());
        }
    }
}
```
Callers change from `merge_indexmap(&mut base.commands, &overlay.commands)` to `merge_indexmap(&mut base.commands, overlay.commands.as_ref())`.
<!-- SECTION:DESCRIPTION:END -->
