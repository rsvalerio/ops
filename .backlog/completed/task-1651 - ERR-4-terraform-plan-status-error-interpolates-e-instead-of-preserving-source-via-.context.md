---
id: TASK-1651
title: >-
  ERR-4: terraform plan status() error interpolates {e} instead of preserving
  source via .context
status: Done
assignee: []
created_date: '2026-06-07 10:53'
updated_date: '2026-06-07 11:32'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:263`

**What**: The non-NotFound arm of the `plan_cmd.status().map_err(...)` closure builds `anyhow::anyhow!("failed to run terraform plan: {e}")`, flattening the `io::Error` into the message string instead of keeping it as the error `source()`.

**Why it matters**: Same convention as wave-140 SEC-21/TASK-1531 — interpolating `{e}` erases the typed cause from the chain, so alternate-style rendering (`{:#}`) and any future `downcast_ref::<io::Error>()` lose information. The fix is `anyhow::Error::new(e).context("failed to run terraform plan")` (the NotFound arm legitimately replaces the message and can stay as-is).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Non-NotFound spawn failure preserves the io::Error as source via .context() instead of {e} interpolation
- [x] #2 NotFound arm's install-hint message is unchanged
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Non-NotFound spawn failure now uses anyhow::Error::new(e).context("failed to run terraform plan"); NotFound install-hint arm unchanged.
<!-- SECTION:NOTES:END -->
