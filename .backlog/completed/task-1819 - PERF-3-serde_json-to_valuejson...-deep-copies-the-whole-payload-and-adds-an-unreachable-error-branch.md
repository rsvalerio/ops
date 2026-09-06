---
id: TASK-1819
title: >-
  PERF-3: serde_json::to_value(json!(...)) deep-copies the whole payload and
  adds an unreachable error branch
status: Done
assignee:
  - TASK-1996
created_date: '2026-08-27 11:33'
updated_date: '2026-08-28 15:13'
labels:
  - code-review-rust
  - idioms-correctness
dependencies: []
modified_files:
  - extensions-rust/create-review-tasks/src/provider.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/create-review-tasks/src/provider.rs:42-46`

**What**:

```rust
serde_json::to_value(serde_json::json!({
    "skill": SKILL_NAME,
    "targets": targets,
}))
.map_err(DataProviderError::from)
```

`serde_json::json!` already evaluates to a `serde_json::Value`. Wrapping it in `serde_json::to_value` re-serializes that `Value` through the `Serialize`/`Deserialize` machinery into a second, structurally identical `Value` — a full deep copy of every target object, string and map allocated twice. `Value`'s `Serialize` impl is infallible, so the `Result` it returns is always `Ok` and the `.map_err(DataProviderError::from)` branch is dead code that no test can reach.

The pattern was presumably copied from `extensions-rust/about/src/units.rs:169` (`serde_json::to_value(&units).map_err(DataProviderError::from)`), where the argument is a `Vec<ProjectUnit>` and both the conversion and its error branch are real. Here the argument is already a `Value`.

**Why it matters**: the allocation is proportional to the workspace member count and buys nothing, and the unreachable `map_err` misleads a reader into thinking payload construction can fail here — it cannot, which also means the `Serialization` variant is undocumented-but-unreachable on this provider. `Ok(serde_json::json!({ ... }))` is the whole function tail.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The redundant serde_json::to_value wrapper is removed; the json! Value is returned directly
- [x] #2 The unreachable map_err(DataProviderError::from) branch is gone
- [x] #3 Existing provider tests still pass unchanged
<!-- AC:END -->
