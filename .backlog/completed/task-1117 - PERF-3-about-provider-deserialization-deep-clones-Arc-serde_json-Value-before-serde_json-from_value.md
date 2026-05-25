---
id: TASK-1117
title: >-
  PERF-3: about provider deserialization deep-clones Arc<serde_json::Value>
  before serde_json::from_value
status: Done
assignee: []
created_date: '2026-05-07 22:07'
updated_date: '2026-05-08 06:29'
labels:
  - code-review-rust
  - perf
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/lib.rs:139`, `extensions/about/src/providers.rs:46`

**What**: Both call sites take `Ok(value)` where `value: Arc<serde_json::Value>` and pass `(*value).clone()` into `serde_json::from_value::<T>(...)`. `from_value` takes `Value` by value, but the surrounding code already holds the only handle to the Arc payload that's needed; the eager `.clone()` performs a deep clone of the entire JSON tree just to feed the deserializer. Switching to `serde_json::from_value::<T>(Value::clone(&value))` is no better — the right fix is to keep the JSON as bytes/string in the data layer or to refactor `from_value` callers to use `serde_path_to_error::deserialize` or `T::deserialize(&*value)` via `serde::Deserialize`.

**Why it matters**: This runs on every `ops about` invocation and once per about subpage provider (coverage/deps/units), so the clone cost scales with both project size (large dependency graphs serialize to large JSON) and number of subpages. PERF-3 specifically targets `.clone()` calls that exist only to satisfy an API signature when a borrow would do. The simplest equivalent fix without changing the data API: `T::deserialize(value.as_ref())` from `serde::Deserialize` avoids the deep clone entirely.

<!-- scan confidence: high; both call sites verified at the listed lines -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Replace (*value).clone() with a borrow-based deserialize (e.g., T::deserialize(value.as_ref())) at both call sites
- [x] #2 Confirm no Value clone occurs on the about hot path under cargo flamegraph or a quick benchmark
- [x] #3 All existing about / providers tests still pass under cargo test --workspace
<!-- AC:END -->
