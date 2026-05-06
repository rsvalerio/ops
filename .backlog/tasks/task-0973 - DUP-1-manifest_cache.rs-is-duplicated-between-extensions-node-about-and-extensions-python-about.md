---
id: TASK-0973
title: >-
  DUP-1: manifest_cache.rs is duplicated between extensions-node/about and
  extensions-python/about
status: To Do
assignee:
  - TASK-1012
created_date: '2026-05-04 21:57'
updated_date: '2026-05-06 06:48'
labels:
  - code-review-rust
  - DUP
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Files**:
- `extensions-node/about/src/manifest_cache.rs:1-67`
- `extensions-python/about/src/manifest_cache.rs:1-97`

**What**: The two `manifest_cache` modules are structurally identical: same `OnceLock<Mutex<HashMap<PathBuf, Option<Arc<str>>>>>` shape, same `CACHE_MAX_ENTRIES = 1024` cap, same poison-recovery branch with the same warn message ("<X> cache mutex was poisoned by a prior panic; recovered"), same cap-then-clear strategy, same `read_optional_text` dispatch. The only differences are the filename literal (`package.json` vs `pyproject.toml`) and the breadcrumb text. The Go crate will need an identical third copy as soon as it grows a sister provider (TASK-0931 already showed this pattern is replicating).

**Why it matters**: Three concerns multiply per copy: (a) the poison-recovery wording / cap value drift between sites (already started — node says "package.json cache" while python says "pyproject cache"); (b) any future fix (TASK-0962 ARCH-2 one-shot poison signal, TASK-0867 cap policy change, switching to LRU) must be made N times and may regress in one copy silently; (c) tests are duplicated in lockstep (`second_call_returns_same_arc`, `arc_is_shared_across_two_consumer_parses`, `poison_recovery_keeps_cache_usable`). Extracting a `ops_about::manifest_cache::ArcTextCache::new("package.json")` (or generic `read_text_cached(root, filename)`) consolidates the policy in one place.

<!-- scan confidence: high — diff between the two files is < 10 lines of substantive logic -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Shared manifest-text cache lives in a single ops_about module
- [ ] #2 Node and Python providers call into the shared cache with a filename argument
- [ ] #3 Existing tests (Arc identity, poison recovery, cap clear) cover the shared implementation, not per-crate copies
<!-- AC:END -->
