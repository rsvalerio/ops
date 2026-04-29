---
id: TASK-0584
title: >-
  ERR-4: collect_compiled_extensions silently drops extensions whose factory
  returns None
status: Done
assignee:
  - TASK-0639
created_date: '2026-04-29 05:17'
updated_date: '2026-04-29 10:58'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry.rs:24`

**What**: `collect_compiled_extensions` does `EXTENSION_REGISTRY.iter().filter_map(|f| f(config, workspace_root))`. A factory returning None (prerequisites not met) is dropped silently — no tracing event indicates a linked-in extension declined to construct. From operator view, an extension that compiled in but quietly opts out is indistinguishable from one that does not exist.

**Why it matters**: ERR-4/debuggability. Single-line tracing::debug lets RUST_LOG=ops=debug answer "the X extension is not running for me".
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Factories that return None log at tracing::debug! with source location or registration name
- [x] #2 No behaviour change for success path
<!-- AC:END -->
