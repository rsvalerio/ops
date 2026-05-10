---
id: TASK-0686
title: >-
  PATTERN-1: go.mod parse_replace_directive rejects absolute and parent-relative
  local replace targets
status: Done
assignee:
  - TASK-0736
created_date: '2026-04-30 05:16'
updated_date: '2026-04-30 09:57'
labels:
  - code-review-rust
  - patterns
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/go_mod.rs:64-75`

**What**: `parse_replace_directive` accepts a target only if it starts with `./` or `.\\` — but cmd/go also accepts `replace foo => /abs/path` and `replace foo => ../shared`; the latter is legitimate in monorepos. Both are silently dropped here. The sibling `go.work` parser does surface `../shared` (modules.rs:33-41) — cross-stack inconsistency in what counts as "local".

**Why it matters**: Go monorepos sometimes use `replace … => /workspace/sub` (absolute, hermetic-build settings) or `=> ../shared`; both render as "not a local replace" and skew module_count.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 parse_replace_directive returns Some(target) for ./, .\\, ../, ..\\, or any path containing a path separator that is not a versioned module coordinate (e.g. no space-separated vX.Y.Z)
- [x] #2 Test asserts replace foo => ../shared and replace foo => /abs/path both populate local_replaces
<!-- AC:END -->
