---
id: TASK-1769
title: >-
  PERF-3: normalize_urls builds a second full-size HashMap whose only consumer
  is the collision warn
status: Done
assignee:
  - TASK-1992
created_date: '2026-08-27 11:20'
updated_date: '2026-08-28 20:06'
labels:
  - code-review-rust
  - performance
dependencies: []
modified_files:
  - extensions-python/about/src/lib.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:317-346`

**What**: The function maintains two `HashMap`s with identical key sets, both pre-sized to `urls.len()`:

```rust
let mut out: HashMap<String, &String> = HashMap::with_capacity(urls.len());
let mut first_seen_raw: HashMap<String, &String> = HashMap::with_capacity(urls.len());
...
out.insert(norm.clone(), v);
first_seen_raw.insert(norm, k);
```

`first_seen_raw` is read on exactly one line — `lib.rs:327`, inside the collision branch — to recover the raw key for the `first_key` field of a `tracing::warn!`. On the overwhelmingly common no-collision path it is written once per URL, never read, and dropped. Every insert also forces `norm.clone()`, an allocation that exists only because the same key must land in both maps.

A single `HashMap<String, (&String, &String)>` mapping normalised key → `(raw_key, url)` carries the same information with one map, one allocation per key, and no clone. The `.copied().map_or("", ...)` recovery dance at `lib.rs:328-330` — which silently substitutes `""` for a key that is structurally guaranteed to be present — disappears with it.

**Why it matters**: `[project.urls]` maps are small, so the wall-clock cost is not the point; the redundant map is a correctness-adjacent smell. Two containers that must stay in lockstep can drift, and the `map_or("")` fallback is the code admitting it cannot prove they haven't — it would degrade the diagnostic to a blank `first_key` rather than fail loudly. Collapsing to one map makes the invariant structural.

Related: the collision branch this code serves has no test coverage at all — filed separately as TASK-1757.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 normalize_urls uses a single map keyed by the normalised key, holding both the raw key and the URL
- [x] #2 The per-key norm.clone() is gone
- [x] #3 The .map_or("", ...) fallback for a missing first-seen key is gone — the raw key comes directly from the entry
- [x] #4 Collision behaviour (keep first, warn with both raw keys and both URLs) is unchanged
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
`normalize_urls` now returns `HashMap<String, (&String, &String)>` -- one map
holding the first-seen raw key and its URL. `first_seen_raw`, the per-key
`norm.clone()`, and the `.copied().map_or("", ...)` fallback are all gone; the
warn reads `first_key` straight out of the entry, so the "both halves are
present" invariant is structural. `pick_url`'s signature follows the new type
and destructures `(_, v)`.

Collision behaviour is unchanged and is now covered by TASK-1757's tests,
which were written against this shape and verified to fail against a naive
`.collect()`.
<!-- SECTION:NOTES:END -->
