---
id: TASK-2374
title: 'READ-13: module and item docs narrate change history instead of end state'
status: To Do
assignee: []
created_date: '2026-10-04 14:11'
updated_date: '2026-10-04 14:52'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2418'
modified_files:
  - extensions-rust/about/src/manifest.rs
  - extensions-rust/about/src/manifest_cache.rs
  - extensions-rust/about/src/members.rs
  - extensions-rust/about/src/coverage_provider.rs
  - extensions-rust/about/src/units.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/about/src:module-docs'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
<!-- scan confidence: candidates to inspect -->

**File**: `extensions-rust/about/src/` (several files)

**What**: Doc comments describe how the code got here rather than what it is. Candidates:
- `manifest.rs:20-37` (`LoadedManifest` doc: "Before TASK-1076 ... overwrote ...", the removed `Deref`)
- `manifest_cache.rs:45-110` (`//!` block: "previous thread-local", "re-deserialized it every time", a test-serialization post-mortem)
- `manifest_cache.rs` `lock_typed_manifest_cache` doc ("previous OnceLock-gated log fired only once")
- `members.rs:50-75` (`ExcludeSet`: "this module used to apply exclude as ...", "no longer an equality test")
- `members.rs` `normalize_exclude_prefix` / `normalize_member` docs (bug post-mortems)
- `coverage_provider.rs:17-60` (`ProjectCoverageCache`: "Earlier this used ptr as usize ... ABA", "Pre-fix the outer mutex ...")
- `units.rs` `resolve_dep_count` doc ("extracted from provide's map closure", "no longer needed")

**Why it matters**: READ-13: these are process artifacts a reader using the module does not need and they go stale; the enduring facts (cache contract, invariants, keys, bounds) should stay, the journey belongs in commit/PR history.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Listed docs describe current behavior and invariants only; migration/previously/pre-fix narration removed
- [ ] #2 Cache contracts (key, value, bound, invalidation) remain documented
<!-- AC:END -->
