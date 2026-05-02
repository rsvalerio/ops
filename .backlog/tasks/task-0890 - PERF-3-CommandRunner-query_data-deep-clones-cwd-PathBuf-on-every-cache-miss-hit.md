---
id: TASK-0890
title: >-
  PERF-3: CommandRunner::query_data deep-clones cwd PathBuf on every cache
  miss/hit
status: Done
assignee: []
created_date: '2026-05-02 09:39'
updated_date: '2026-05-02 11:10'
labels:
  - code-review-rust
  - PERF
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: crates/runner/src/command/mod.rs:234

**What**: `CommandRunner::query_data` constructs a fresh `Context` on every invocation via `ops_extension::Context::new(Arc::clone(&self.config), (*self.cwd).clone())`. The `(*self.cwd).clone()` dereferences the `Arc<PathBuf>` and deep-clones the inner `PathBuf` — a heap allocation and string copy on every call. The runner already wraps cwd in an `Arc<PathBuf>` precisely so the parallel-exec hot path (build.rs / TASK-0462) can share it via cheap `Arc::clone`. The data path drops out of that invariant: each provider lookup pays a `PathBuf` allocation regardless of whether the cache hits.

**Why it matters**: This regresses the OWN-2 invariant TASK-0462 established for parallel exec, and shows up in `ops about` / `ops deps` paths that touch many providers. The fix is to thread `Arc<PathBuf>` through `Context::new` (additive method or a new constructor) so the data path matches the exec path. Note TASK-0621 already addressed *per-provider* working_directory clones inside `provide` impls; this is the upstream call site that hands those impls an already-cloned path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Context exposes a constructor that accepts Arc<PathBuf> without re-allocating the inner PathBuf
- [ ] #2 CommandRunner::query_data uses the Arc-only path; existing PathBuf-by-value Context::new is preserved for backwards compatibility
- [ ] #3 A trace event or test pins that strong_count > 1 on the cwd Arc after query_data returns, mirroring the exec-path proof in build_command_async
<!-- AC:END -->
