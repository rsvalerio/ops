---
id: TASK-2256
title: 'API-16: apply_with_prefix left as caller-less public surface after the gated variant replaced its callers'
status: Triage
assignee: []
created_date: '2026-09-09 18:47'
labels:
  - code-review-rust
  - api-design
dependencies: []
modified_files:
  - crates/theme/src/style/sgr.rs
  - crates/theme/src/style.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/style/sgr.rs:109` (re-exported at `crates/theme/src/style.rs:23`)

**What**: PERF-3 / TASK-2082 moved every render entry point in ops-theme onto `apply_with_prefix_gated`, resolving the colour gate once per entry point. After that change `apply_with_prefix` — the convenience form that calls `color_enabled()` (a live `NO_COLOR` env read) per invocation — has zero callers in the workspace. It remains `pub` in `ops-theme` with a doc comment positioning it as the convenience form for cold paths, but no cold path currently uses it.

**Why it matters**: it is dead public API: untested through any caller, and a standing invitation for a future hot-path caller to pick the eager-gate form by accident — the exact pattern TASK-2082 removed. Either delete it (and its `style.rs` re-export) or give it a caller worth keeping; a workspace-internal crate with no external consumers makes deletion cheap if triage agrees the convenience form is not worth keeping.

**Origin**: discovered during TASK-2244 while fixing TASK-2082 (orphan created by the fix; public API so removal was deferred to triage per wave protocol).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 apply_with_prefix is either removed from the ops-theme public API or has a real caller that justifies the eager-gate convenience form
<!-- AC:END -->
