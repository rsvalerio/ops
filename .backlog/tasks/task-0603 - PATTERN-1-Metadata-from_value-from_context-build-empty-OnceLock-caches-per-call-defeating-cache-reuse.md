---
id: TASK-0603
title: >-
  PATTERN-1: Metadata::from_value/from_context build empty OnceLock caches per
  call, defeating cache reuse
status: Triage
assignee: []
created_date: '2026-04-29 05:19'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/types.rs:101`

**What**: TASK-0477 added member-id OnceLock caches inside Metadata, but Metadata::from_context constructs a new Metadata (with fresh empty OnceLocks) every call even though inner is shared via Arc. Each call site pays the HashSet build at least once. The original optimisation only pays off if a single Metadata instance is reused for many members()/is_member() calls.

**Why it matters**: Cache lives on the wrapper, not on the underlying Arc<Value>. Document the actual performance contract or move the caches behind the Arc so they are shared.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Doc-comment on Metadata::from_context clarifies cache lifetime, OR HashSets move into structure shared via Arc
<!-- AC:END -->
