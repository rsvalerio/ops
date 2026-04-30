---
id: TASK-0660
title: 'SEC-25: Stack::detect uses exists()-then-decide for stack manifest probing'
status: To Do
assignee:
  - TASK-0739
created_date: '2026-04-30 05:12'
updated_date: '2026-04-30 06:07'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/stack.rs:107-118`

**What**: `current.join(f).exists()` walks parents using `exists()`-then-decide for stack manifest detection.

**Why it matters**: Classic check-then-use shape. Severity is low — worst outcome is a wrong stack default for one CLI invocation, files are well-known names. Flagging because the security-relevant path (build.rs resolve_spec_cwd) was already hardened; codebase consistency matters.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace exists() with try_exists()/metadata() and treat Err as 'not found' with tracing::debug log on permission errors
<!-- AC:END -->
