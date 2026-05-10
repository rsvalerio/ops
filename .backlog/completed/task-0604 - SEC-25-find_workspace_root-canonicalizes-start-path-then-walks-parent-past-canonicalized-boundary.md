---
id: TASK-0604
title: >-
  SEC-25: find_workspace_root canonicalizes start path then walks parent() past
  canonicalized boundary
status: Done
assignee:
  - TASK-0645
created_date: '2026-04-29 05:19'
updated_date: '2026-04-29 17:47'
labels:
  - code-review-rust
  - SEC
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/lib.rs:277`

**What**: start is canonicalized once; the walk uses current.parent() each iteration. manifest_declares_workspace re-reads parent`s Cargo.toml without re-canonicalising — if a parent directory is itself a symlink (monorepos with worktrees, ~/code -> /Volumes/work), the walk follows the symlinked parent chain rather than the resolved one. Combined with MAX_ANCESTOR_DEPTH = 64, bounded but ambiguity remains.

**Why it matters**: Doc comment claims walk is symlink-safe; not in the general case. Either fix the walk to canonicalise each ancestor or reword the contract.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either canonicalise each ancestor, OR weaken doc to acknowledge partial guarantee
- [ ] #2 Test covers a symlinked parent directory case
<!-- AC:END -->
