---
id: TASK-0345
title: 'DUP-3: load_config error-recovery boilerplate duplicated across handlers'
status: Done
assignee:
  - TASK-0420
created_date: '2026-04-26 09:34'
updated_date: '2026-04-27 11:32'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/main.rs:110-119`, `crates/cli/src/about_cmd.rs:31-40`, `crates/cli/src/hook_shared.rs:27-35`

**What**: Three handlers repeat the same shape: match ops_core::config::load_config() { Ok(c) => c, Err(e) => { ui::warn(...); Config::default() } }. The wording differs slightly between sites.

**Why it matters**: DUP-3 (3+ occurrences). Centralizing keeps the warn copy uniform and makes adding telemetry/structured logging a one-touch change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add a helper (in cli or ops_core::config) that returns Config after logging via tracing::warn! + ui::warn with a caller-supplied context tag
- [ ] #2 Replace all three sites with the helper
<!-- AC:END -->
