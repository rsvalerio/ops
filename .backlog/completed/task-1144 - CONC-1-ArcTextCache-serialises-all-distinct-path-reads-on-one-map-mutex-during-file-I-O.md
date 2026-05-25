---
id: TASK-1144
title: >-
  CONC-1: ArcTextCache serialises all distinct-path reads on one map mutex
  during file I/O
status: Done
assignee:
  - TASK-1261
created_date: '2026-05-08 07:42'
updated_date: '2026-05-08 14:43'
labels:
  - code-review-rust
  - CONC
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/manifest_cache.rs:91-154`

**What**: TASK-1051 fix holds the cache mutex across `read_optional_text(...)` so racing readers of the same uncached path observe one Arc. Side effect: readers of completely unrelated paths also block on the same global mutex during disk I/O (up to MAX_MANIFEST_BYTES = 4 MiB). On a multi-stack workspace warming Node + Python providers in parallel against many distinct package roots, this collapses concurrent reads to single-threaded.

**Why it matters**: The dedup contract only needs to serialise same-path readers, not all readers. Current shape is correct but pessimistic; under daemon hosts (LSP/watchers) the cap-eviction path additionally amplifies serialised reads.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Use a per-key once-cell pattern (HashMap<PathBuf, Arc<OnceLock<Option<Arc<str>>>>>) so distinct paths progress in parallel while same-path readers still observe one Arc
- [ ] #2 Add a regression test where two threads read distinct uncached paths and complete without blocking on each other
<!-- AC:END -->
