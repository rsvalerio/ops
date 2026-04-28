---
id: TASK-0445
title: 'ERR-1: gather_available_commands silently swallows builtin_extensions error'
status: Done
assignee:
  - TASK-0536
created_date: '2026-04-28 05:43'
updated_date: '2026-04-28 16:12'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/hook_shared.rs:107`

**What**: `if let Ok(exts) = crate::registry::builtin_extensions(config, cwd) { ... }` discards the error. A misconfigured `extensions.enabled` (e.g. a typo) results in an empty multiselect during `run-before-commit install` with no diagnostic.

**Why it matters**: A typoed extension name produces an empty/short selection list indistinguishable from "no extensions configured", giving the user no path to discovery.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Log at tracing::warn! (with the error) when the extension registry build fails; ideally surface via ops_core::ui::warn
- [x] #2 Test confirms a misconfigured extensions.enabled produces a warning, not silent omission
<!-- AC:END -->
