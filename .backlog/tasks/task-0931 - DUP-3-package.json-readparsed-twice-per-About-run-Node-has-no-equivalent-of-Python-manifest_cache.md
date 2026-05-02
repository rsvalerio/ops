---
id: TASK-0931
title: >-
  DUP-3: package.json read+parsed twice per About run; Node has no equivalent of
  Python manifest_cache
status: Done
assignee: []
created_date: '2026-05-02 15:33'
updated_date: '2026-05-02 17:28'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/units.rs:79-99`; `extensions-node/about/src/package_json.rs:78-92`

**What**: The Node identity provider calls `parse_package_json(root)` and the Node units provider independently calls `read_optional_text(&pkg_path, ...)` plus `serde_json::from_str::<RawRoot>` on the same `package.json`. Python solved the analogous duplication in TASK-0816 by introducing `extensions-python/about/src/manifest_cache.rs`; Node has no equivalent.

**Why it matters**: Every About invocation pays double IO + double JSON parse for the root `package.json` (typically the largest manifest in a JS monorepo, often 5–50 KB with rich dependency trees). Mirrors PERF-3 / TASK-0854 motivation. Also a structural-consistency gap: Python and Node providers should share the same per-root cache pattern.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A manifest_cache (or shared abstraction in ops_about) caches the raw package.json text per root, returned as Arc<str>, used by both parse_package_json and workspace_member_globs.
- [x] #2 Both consumers serde_json::from_str directly off the cached Arc<str> (no toml::Value-style deep clone).
- [x] #3 Cache is bounded (matches CACHE_MAX_ENTRIES discipline from TASK-0867) and recovers from mutex poisoning (matches TASK-0878).
- [x] #4 Test proves the second call returns the same Arc (matches the second_call_returns_same_arc pattern in manifest_cache.rs).
<!-- AC:END -->
