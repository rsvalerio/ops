---
id: TASK-0270
title: >-
  ERR-5: run_about_setup_with calls load_config().unwrap_or_default in
  production
status: Done
assignee: []
created_date: '2026-04-23 06:36'
updated_date: '2026-04-23 14:29'
labels:
  - rust-code-review
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/about_cmd.rs:26`

**What**: Malformed .ops.toml silently yields empty currently_enabled; user may save reset defaults over real config.

**Why it matters**: Silent config masking outside tests.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Propagate error or warn explicitly
- [ ] #2 Only default on NotFound
<!-- AC:END -->
