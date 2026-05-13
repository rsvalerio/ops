---
id: TASK-1403
title: >-
  READ-5: load_global_config_at silently shadows legacy bare-extension file when
  canonical sibling exists
status: To Do
assignee:
  - TASK-1453
created_date: '2026-05-13 18:10'
updated_date: '2026-05-13 19:09'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader.rs:369-386`

**What**: When both `<base>.toml` and the legacy bare `<base>` are present, the bare file is silently ignored — only a `debug!` event records the choice.

**Why it matters**: Operators who left a stale legacy file in place see no signal that their edits are being ignored. A `tracing::warn!` (or one-shot stderr nudge) would surface the situation at the level it deserves without breaking the existing precedence rule.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 When both canonical and legacy config paths exist, the user sees a warn-level signal
<!-- AC:END -->
