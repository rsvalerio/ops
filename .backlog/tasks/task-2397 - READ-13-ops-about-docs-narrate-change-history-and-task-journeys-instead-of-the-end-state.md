---
id: TASK-2397
title: 'READ-13: ops-about docs narrate change history and task journeys instead of the end state'
status: Done
assignee: []
created_date: '2026-10-04 14:15'
updated_date: '2026-10-04 15:54'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2421'
modified_files:
  - extensions/about/src/workspace.rs
  - extensions/about/src/manifest_cache.rs
  - extensions/about/src/manifest_io.rs
  - extensions/about/src/lib.rs
  - extensions/about/src/lru.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/about/src:module-docs'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/workspace.rs:1-15`, `extensions/about/src/workspace.rs:26-40`, `extensions/about/src/manifest_cache.rs:1-60`, `extensions/about/src/manifest_io.rs:1-15`, `extensions/about/src/lib.rs` (`resolve_identity`, `warm_generic_providers`, `enrich_from_db`), `extensions/about/src/lru.rs`

**What**: Module and item docs (`//!`, `///`) describe how the code got here: "the previous behaviour silently flattened...", "the body used to inline five concerns... across 136 lines", "The previous policy cleared the entire map", "Six copies meant the next copy/paste would silently drift", "now warn-logged ... Previously ...", plus TASK-NNNN ids (about 170 across the crate; workspace.rs 36, manifest_cache.rs 38, lru.rs 26). This is migration/design-journal text, not API documentation. Enduring invariants (fail-closed excludes, containment invariant, no-TTL freshness policy) are legitimate and should stay, stated as present-tense behaviour.

<!-- scan confidence: candidates to inspect -->

**Why it matters**: Readers of the API get history they cannot use, and it goes stale on the next change while looking authoritative (READ-13). It belongs in commit messages or the backlog.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Docs in the listed files state current behaviour and invariants only; 'previously/used to/now' narration and TASK ids are removed
- [x] #2 Enduring design properties (fail-closed exclude, containment invariant, no-TTL cache policy) are kept in present tense

<!-- AC:END -->
