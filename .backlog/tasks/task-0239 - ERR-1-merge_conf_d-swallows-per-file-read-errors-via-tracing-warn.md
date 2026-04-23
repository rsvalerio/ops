---
id: TASK-0239
title: 'ERR-1: merge_conf_d swallows per-file read errors via tracing::warn'
status: Done
assignee: []
created_date: '2026-04-23 06:35'
updated_date: '2026-04-23 14:29'
labels:
  - rust-code-review
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader.rs:112`

**What**: On a parse or IO error for a single .ops.d/*.toml, the function warns and continues, silently dropping intentional overlay config.

**Why it matters**: Users misconfiguring one overlay file get surprising silent behavior with no CI failure.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Fail fast with the file path in context on any read_config_file error
- [ ] #2 Add a test asserting parse errors in one overlay file abort load_config
<!-- AC:END -->
