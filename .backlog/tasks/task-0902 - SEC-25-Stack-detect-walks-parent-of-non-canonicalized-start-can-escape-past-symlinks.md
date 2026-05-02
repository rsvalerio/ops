---
id: TASK-0902
title: >-
  SEC-25: Stack::detect walks parent() of non-canonicalized start, can escape
  past symlinks
status: Triage
assignee: []
created_date: '2026-05-02 10:09'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/stack.rs:157`

**What**: Stack::detect takes start as Path, does start.to_path_buf() then walks via current.pop() up to MAX_DETECT_DEPTH=64. It never canonicalizes start, so a symlink chain within the cwd makes pop() yield ancestors that lexically appear inside the workspace but are outside the canonical workspace boundary. The chosen stack default commands then run against an unrelated parent project's manifests.

**Why it matters**: Detection state silently leaks across canonical-workspace boundaries when the cwd is reached through a symlink.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Stack::detect canonicalizes start once before the parent walk, with a tracing::debug breadcrumb when canonicalize fails so detection falls back to lexical walk
- [ ] #2 Test that a symlinked cwd resolves to the same Stack as the canonical path
<!-- AC:END -->
