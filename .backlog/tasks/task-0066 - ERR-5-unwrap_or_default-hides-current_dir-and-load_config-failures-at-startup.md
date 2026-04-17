---
id: TASK-0066
title: 'ERR-5: unwrap_or_default hides current_dir and load_config failures at startup'
status: Done
assignee: []
created_date: '2026-04-17 11:30'
updated_date: '2026-04-17 15:41'
labels:
  - rust-codereview
  - err
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/main.rs:89`

**What**: `early_config = load_config().unwrap_or_default()` and `current_dir().unwrap_or_default()` silently swallow IO/parse errors; detected_stack is computed from a possibly-empty PathBuf.

**Why it matters**: A malformed .ops.toml or CWD-permission error causes ops to silently fall back to defaults, producing surprising help output instead of a clear diagnostic.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Log a tracing::warn! when load_config fails
- [ ] #2 Bail out or emit stderr message when current_dir() returns Err
<!-- AC:END -->
