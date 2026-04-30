---
id: TASK-0659
title: >-
  API-2: Arc-only invariant in build_command_async is enforced only by
  convention
status: To Do
assignee:
  - TASK-0740
created_date: '2026-04-30 05:12'
updated_date: '2026-04-30 06:07'
labels:
  - code-review-rust
  - api-design
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/build.rs:228-266`

**What**: The "Arc-only" invariant (no deep clone of cwd/vars on the parallel hot path) is enforced by convention and a `tracing::trace!` strong-count probe rather than the type system.

**Why it matters**: TASK-0462 documents the intent at length, but any future caller that constructs an owned `PathBuf` / `Variables` and wraps fresh in `Arc::new(...)` per spawn re-introduces the regression. The trace-event "test" only fires under `RUST_LOG=trace` and isn't asserted in CI.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Wrap shared inputs in newtypes (SharedCwd(Arc<PathBuf>), SharedVars(Arc<Variables>)) constructed once at runner setup and passed by clone(); the type system then guarantees Arc::clone-only sharing
- [ ] #2 Or add a #[track_caller] debug-assert: debug_assert!(Arc::strong_count(&cwd) > 1, "cwd Arc must be shared") on a parallel spawn
<!-- AC:END -->
