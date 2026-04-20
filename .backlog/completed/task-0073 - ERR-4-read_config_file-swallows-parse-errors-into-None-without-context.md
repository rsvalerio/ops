---
id: TASK-0073
title: 'ERR-4: read_config_file swallows parse errors into None without context'
status: Done
assignee: []
created_date: '2026-04-17 11:32'
updated_date: '2026-04-17 15:30'
labels:
  - rust-codereview
  - err
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader.rs:59`

**What**: read_config_file returns Option<ConfigOverlay>: both NotFound and parse errors map to None; parse errors are logged but never propagated to callers.

**Why it matters**: Callers cannot distinguish file-not-present from file-is-malformed; invalid configs are silently treated as missing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Change return type to anyhow::Result<Option<ConfigOverlay>> with .with_context(...)
- [ ] #2 Callers explicitly handle parse errors (fail fast or warn once)
<!-- AC:END -->
