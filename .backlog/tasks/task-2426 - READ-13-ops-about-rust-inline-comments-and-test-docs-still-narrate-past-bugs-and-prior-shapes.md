---
id: TASK-2426
title: 'READ-13: ops-about-rust inline comments and test docs still narrate past bugs and prior shapes'
status: To Do
assignee: []
created_date: '2026-10-04 15:16'
updated_date: '2026-10-10 14:31'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2439'
modified_files:
  - extensions-rust/about/src/manifest.rs
  - extensions-rust/about/src/manifest_cache.rs
  - extensions-rust/about/src/members.rs
  - extensions-rust/about/src/units.rs
  - extensions-rust/about/src/coverage_provider.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/manifest.rs:70`, `extensions-rust/about/src/manifest_cache.rs:138`, `extensions-rust/about/src/members.rs:64`, `extensions-rust/about/src/units.rs:374`, `extensions-rust/about/src/coverage_provider.rs:350`

**What**: TASK-2374 rewrote the `//!` and `///` item docs in production code. Inline `//` comments and test-module doc comments in the same files still describe the journey: "The previous mutation flattened ...", "the legacy trust-until-refresh behaviour", "the pre-TASK-2040 behaviour", "the resolved list itself was emitted verbatim ... survived the dedup", "The previous shape asserted only ...", "Pre-fix the outer mutex ...", "The previous OnceLock-gated warn fired only on the first poisoning".

**Why it matters**: READ-13: the same stale-prone process narration, one level down from the item docs; a reader has to separate the current invariant from the history of the bug that motivated it.

**Origin**: discovered during TASK-2418 while fixing TASK-2374.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Inline comments and test docs in the listed files state the current invariant or what the test pins, without previous/pre-fix/legacy narration
<!-- AC:END -->
