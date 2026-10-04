---
id: TASK-2393
title: 'CL-3: SHORTNAME doc contradicts its value'
status: To Do
assignee: []
created_date: '2026-10-04 14:15'
updated_date: '2026-10-04 14:52'
labels:
  - code-review-rust
  - clarity
dependencies: []
parent_task_id: 'TASK-2422'
modified_files:
  - extensions/config-checkers/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'CL-3:extensions/config-checkers/src/lib.rs:SHORTNAME'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/lib.rs:51-53`

**What**: The doc on `SHORTNAME` says "CLI-facing short name — the subcommand the user types (`ops check-json`, `ops check-yaml`)", but the value is `"config-checkers"`, which is not a subcommand the user types (the subcommands are `check-json` / `check-yaml`, via `command_names`). The doc and the constant disagree, and `SHORTNAME` is identical to `NAME`, so it is unclear what it is for.

**Why it matters**: A reader relying on the doc will assume a wrong CLI contract. Either correct the doc to say what the shortname is used for (as the other extensions use it, e.g. `db` for sqlite) or change the value if a different one was intended.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 SHORTNAME doc accurately describes how the value is used
- [ ] #2 If the value was meant to be a subcommand, it is corrected and covered by a test
<!-- AC:END -->
