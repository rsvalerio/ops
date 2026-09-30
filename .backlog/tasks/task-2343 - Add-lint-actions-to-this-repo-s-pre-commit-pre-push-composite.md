---
id: TASK-2343
title: 'Add lint-actions to this repo''s pre-commit/pre-push composite'
status: In Progress
assignee: []
created_date: '2026-09-28 16:56'
updated_date: '2026-09-29 17:14'
labels:
  - code-review-rust
  - ci
dependencies: []
modified_files:
  - .ops.toml
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `.ops.toml`

**What**: with the forge allow-list now under `[lint_actions]` in `.ops.toml` (TASK-2337), a local `ops lint-actions` passes, so the check can join a local composite. `verify` itself comes from the shipped rust stack default (`crates/core/src/.default.rust.ops.toml`) and changing it affects every Rust project, so the repo-local candidates are `run-before-commit` or `run-before-push` in `.ops.toml`. Today the policy only runs in CI's Workflow Guard job.

**Why it matters**: a workflow edit that drops a SHA pin or adds `secrets: inherit` is caught only after push.

**Origin**: discovered during TASK-2341 while fixing TASK-2337.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 lint-actions runs in a local composite (run-before-commit or run-before-push), or a note records why it stays CI-only

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
lint-actions appended to run-before-commit in .ops.toml (it compiles nothing and runs in ms, so commit time rather than pre-push). ops --dry-run run-before-commit shows it as the last step; ops lint-actions passes on the tree.

ops side in rsvalerio/ops#81.

<!-- SECTION:NOTES:END -->
