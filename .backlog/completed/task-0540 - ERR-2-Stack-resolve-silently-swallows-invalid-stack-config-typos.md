---
id: TASK-0540
title: 'ERR-2: Stack::resolve silently swallows invalid stack= config typos'
status: Done
assignee:
  - TASK-0639
created_date: '2026-04-29 04:58'
updated_date: '2026-04-29 10:52'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/stack.rs:41`

**What**: `Stack::resolve` does `config_stack.and_then(|s| s.parse().ok()).or_else(detect)`. A typo like `stack = "rast"` parses to `None` and falls through to filesystem detection. The user override is dropped without any diagnostic.

**Why it matters**: Users debugging "wrong stack detected" have no signal that their override was rejected; common surprise on multi-stack repos and CI containers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 An unparseable config.stack value emits tracing::warn! (and a user-visible ui::warn) listing the offending value and accepted stack names before falling back
- [x] #2 Test asserts the warning fires for "not-a-stack" and is silent for accepted values
<!-- AC:END -->
