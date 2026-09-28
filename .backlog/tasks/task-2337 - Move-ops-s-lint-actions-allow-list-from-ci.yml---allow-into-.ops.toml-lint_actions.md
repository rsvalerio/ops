---
id: TASK-2337
title: 'Move ops''s lint-actions allow-list from ci.yml --allow into .ops.toml [lint_actions]'
status: To Do
assignee: []
created_date: '2026-09-28 15:24'
updated_date: '2026-09-28 16:38'
labels:
  - code-review-rust
  - ci
dependencies: []
parent_task_id: 'TASK-2341'
modified_files:
  - .github/workflows/ci.yml
  - .ops.toml
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `.github/workflows/ci.yml`

**What**: the Workflow Guard job passes `--allow rsvalerio/forge/` on the command line. The allow-list belongs under `[lint_actions] allow` in `.ops.toml`, where `ops lint-actions` (and composites listing it, e.g. `verify`) pick it up. It was not put there because ops binaries predating the `[lint_actions]` section reject `.ops.toml` (deny_unknown_fields), which would break every locally installed ops — including concurrent `ops backlog` runners — until reinstall.

**Why it matters**: the exemption lives in CI only, so a local `ops lint-actions` reports the forge refs as violations and the check cannot join `verify`.

**Origin**: discovered during TASK-2334 while fixing TASK-2328.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Once a released ops with [lint_actions] is the installed baseline, .ops.toml sets allow = ["rsvalerio/forge/"], ci.yml drops --allow, and a local ops lint-actions passes
<!-- AC:END -->
