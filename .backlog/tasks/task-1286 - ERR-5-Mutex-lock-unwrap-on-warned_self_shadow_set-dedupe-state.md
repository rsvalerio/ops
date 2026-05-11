---
id: TASK-1286
title: 'ERR-5: Mutex lock unwrap on warned_self_shadow_set dedupe state'
status: To Do
assignee:
  - TASK-1304
created_date: '2026-05-11 15:26'
updated_date: '2026-05-11 16:48'
labels:
  - code-review-rust
  - error
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/extension_cmd.rs:169`

**What**: `warned_self_shadow_set().lock().unwrap()` panics if a prior holder of the lock panicked while mutating the set. The set is touched on the operator-facing `extension show`/`extension list` paths.

**Why it matters**: Poisoning here is benign (the set only tracks whether we already warned about a (ext, cmd) pair), but unwrapping converts a recoverable poisoned-lock into a CLI abort during diagnostics. The repo's discipline elsewhere prefers to recover via `.into_inner()` or `unwrap_or_else(|e| e.into_inner())`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace .lock().unwrap() with .lock().unwrap_or_else(|poisoned| poisoned.into_inner())
- [ ] #2 Add a brief comment noting the set is advisory dedupe state and poisoning is acceptable to ignore
- [ ] #3 Existing dedupe test continues to pass
<!-- AC:END -->
