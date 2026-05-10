---
id: TASK-0795
title: >-
  ARCH-2: TYPED_MANIFEST_CACHE thread_local cache is invisible to
  non-current-thread callers, defeating its purpose under multi-thread runtimes
status: Done
assignee:
  - TASK-0821
created_date: '2026-05-01 05:59'
updated_date: '2026-05-01 06:45'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs:48-94`

**What**: load_workspace_manifest caches the parsed Arc<CargoToml> in a thread_local! keyed by working directory. The doc-comment says identity / units / coverage providers all need this once per ops about. If the about runner ever schedules providers on different tokio worker threads (or swaps to a JoinSet-based fan-out), each worker re-parses the manifest because the thread-local is per-thread, not per-Context.

**Why it matters**: Caching invariant is enforced only by the single-threaded execution shape of the current callers. A future refactor parallelising the about pipeline silently degrades the cache to "off" with no signal.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Move the cache onto Context (e.g. via ctx.cached/get_or_provide keyed on the typed Arc<CargoToml>) or behind a single Mutex<HashMap<PathBuf, Arc<CargoToml>>> if cross-thread sharing is desired
- [ ] #2 Add a unit test that runs load_workspace_manifest from two threads with the same Context and verifies they see the same Arc allocation
- [ ] #3 Document the chosen sharing semantics in the module-level rustdoc
<!-- AC:END -->
