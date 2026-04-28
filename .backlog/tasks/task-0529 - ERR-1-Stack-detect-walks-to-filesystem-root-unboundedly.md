---
id: TASK-0529
title: 'ERR-1: Stack::detect walks to filesystem root unboundedly'
status: To Do
assignee:
  - TASK-0536
created_date: '2026-04-28 06:53'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/stack.rs:60`

**What**: detect pops parents until pop() returns false, with no depth cap. On pathological cwd depths (~thousands of components on FUSE/network mounts) every CLI invocation does a full chain of Path::join + exists syscalls.

**Why it matters**: Cheap on healthy machines but unbounded by design; a small MAX_DEPTH (mirroring expand_to_leaves) guards against degenerate filesystems and accidental symlink loops above the cwd.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add a sane MAX_DEPTH (e.g. 64) and bail out
- [ ] #2 Document the cap
<!-- AC:END -->
