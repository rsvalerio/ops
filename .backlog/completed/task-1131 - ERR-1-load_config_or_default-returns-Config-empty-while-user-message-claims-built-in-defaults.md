---
id: TASK-1131
title: >-
  ERR-1: load_config_or_default returns Config::empty() while user message
  claims built-in defaults
status: Done
assignee:
  - TASK-1268
created_date: '2026-05-08 07:40'
updated_date: '2026-05-09 17:28'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader.rs:184`

**What**: `load_config_or_default` returns `Config::empty()` on failure but the user-visible message at line 191 says \"using built-in defaults\" and the doc comment at line 175-176 promises `Config::default()` semantics. `Config::empty()` carries no commands, themes, or stack — the operator chasing a load failure sees the warning, runs `ops --list`, sees an empty list, and concludes the binary is broken.

**Why it matters**: Misleading diagnostic. TRAIT-4 / TASK-0872 gated `Config::default()` behind test gates to prevent blank-slate fallbacks; this caller's user message did not get updated to match.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either change the user message to honestly report empty config, or load default_ops_toml() so the fallback carries internal defaults
- [ ] #2 Update the doc comment so it stops referencing Config::default()
<!-- AC:END -->
