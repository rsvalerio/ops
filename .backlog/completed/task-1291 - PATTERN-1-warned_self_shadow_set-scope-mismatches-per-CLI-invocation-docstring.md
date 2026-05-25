---
id: TASK-1291
title: >-
  PATTERN-1: warned_self_shadow_set scope mismatches 'per CLI invocation'
  docstring
status: Done
assignee:
  - TASK-1304
created_date: '2026-05-11 16:10'
updated_date: '2026-05-11 18:06'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/extension_cmd.rs:136-142`

**What**: `warned_self_shadow_set` is a process-global `OnceLock<Mutex<HashSet>>` whose docstring claims dedup is \"per CLI invocation\". In a single CLI binary that is true, but the set grows unbounded over the process lifetime and is shared across any in-process reuse (test binaries today; future library embedding).

**Why it matters**: Mismatch between stated scope and actual scope; couples a logging concern to global state and makes the behaviour hard to reset for tests (see companion TEST-1 finding).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Move dedup state into CommandRegistry or a per-call context passed to extension_summary
- [ ] #2 Docstring matches the actual scope of dedup
- [ ] #3 Two distinct CLI invocations in the same process emit warnings independently
<!-- AC:END -->
