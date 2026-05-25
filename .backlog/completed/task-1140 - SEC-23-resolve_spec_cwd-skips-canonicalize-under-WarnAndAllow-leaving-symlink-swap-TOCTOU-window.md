---
id: TASK-1140
title: >-
  SEC-23: resolve_spec_cwd skips canonicalize under WarnAndAllow leaving
  symlink-swap TOCTOU window
status: Done
assignee:
  - TASK-1260
created_date: '2026-05-08 07:41'
updated_date: '2026-05-08 14:03'
labels:
  - code-review-rust
  - SEC
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/build.rs:387-395`

**What**: Under `Deny`, `resolve_spec_cwd` returns the canonical path so chdir does not re-resolve symlinks (TASK-0773). Under `WarnAndAllow` the same code returns `ep.clone()` or `joined` verbatim — symlinks are re-resolved by the kernel at exec time. Asymmetry leaves the interactive path exposed to symlink swap between `detect_workspace_escape`'s canonicalize and the spawn.

**Why it matters**: SEC-25 already pays the canonicalize cost; reusing it under WarnAndAllow costs nothing and closes the same TOCTOU window for interactive users. The trust-model rationale (.ops.toml is trusted) explains why the escape itself is allowed, not why a symlink-swap race is allowed to redirect the child.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Apply canonicalize-on-success uniformly under both policies (best-effort fallback when canonicalize fails)
- [x] #2 Update CwdEscapePolicy::Deny doc so the residual TOCTOU note no longer claims WarnAndAllow is strictly weaker
<!-- AC:END -->
