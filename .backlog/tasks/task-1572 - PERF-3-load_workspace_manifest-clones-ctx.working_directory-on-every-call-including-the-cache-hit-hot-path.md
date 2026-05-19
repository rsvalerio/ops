---
id: TASK-1572
title: >-
  PERF-3: load_workspace_manifest clones ctx.working_directory on every call,
  including the cache-hit hot path
status: Done
assignee:
  - TASK-1578
created_date: '2026-05-19 16:35'
updated_date: '2026-05-19 18:48'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs:281,315`

**What**: `load_workspace_manifest` opens with `let cwd: PathBuf = PathBuf::clone(&ctx.working_directory);` unconditionally. Every cache-hit call (the dominant case — identity, units, coverage, deps all hit the cache after the first miss) then clones `cwd` again on line 315 (`guard.victim_queue.push(tick, cwd.clone())`). On the cold path the third use at line 390/391 takes ownership for the HashMap insert.

So a typical `ops about` run pays:
- 4 providers × 1 cache-hit each ≈ 8 `PathBuf` clones for a value already borrowed via `ctx.working_directory`.

The hot path only needs a `&Path` for the HashMap key lookup and the freshness stat; only the LRU push and the cold-path insert need an owned `PathBuf`. Pre-fix the cache hit path can take the cwd as `&Path` and clone only when actually pushing the victim entry — and even that push could happen once per *insert* rather than once per *hit*.

**Why it matters**: Small per-call alloc cost but the about pipeline is the canonical example used to justify the sibling caches (TASK-1201, TASK-1195, TASK-0969). The clone-on-every-hit is the only remaining un-amortised allocation in the hot read path; cleaning it up keeps the pipeline's "boring fast" contract intact and removes one more `Path → PathBuf` round-trip from the critical section.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Cache-hit path in load_workspace_manifest does not own a PathBuf when only a &Path is needed (HashMap lookup, freshness stat)
- [ ] #2 Victim-queue push (line 315) avoids cloning cwd on every cache hit — push at most once per insert, or rework the LRU update so hit-tick refresh does not require a fresh PathBuf clone
- [ ] #3 typed_manifest_cache_returns_same_arc_then_invalidates_on_refresh and resolved_workspace_members_are_amortised_via_typed_manifest_cache still pass without modification
<!-- AC:END -->
