---
id: TASK-0222
title: 'SEC-25: ensure_config_command uses check-then-read on .ops.toml (TOCTOU)'
status: To Do
assignee: []
created_date: '2026-04-23 06:33'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/lib.rs:117`

**What**: `config_path.exists()` then `read_to_string` is racy — file can be swapped/removed between calls.

**Why it matters**: Can cause confusing errors or read content from a different file in a shared repo.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Attempt read directly and treat NotFound as empty
- [ ] #2 Document or remove the exists() check
<!-- AC:END -->
