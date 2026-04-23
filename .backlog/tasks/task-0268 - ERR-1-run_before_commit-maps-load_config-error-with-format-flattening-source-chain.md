---
id: TASK-0268
title: >-
  ERR-1: run_before_commit maps load_config error with format! flattening source
  chain
status: Done
assignee: []
created_date: '2026-04-23 06:36'
updated_date: '2026-04-23 07:48'
labels:
  - rust-code-review
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/subcommands.rs:102`

**What**: `.map_err(|e| anyhow!("...: {e}"))` drops downcast; same pattern at line 139 for push.

**Why it matters**: Context should compose via anyhow, not replace.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Replace with .context(...)
- [x] #2 Preserve chain for both hook handlers
<!-- AC:END -->
