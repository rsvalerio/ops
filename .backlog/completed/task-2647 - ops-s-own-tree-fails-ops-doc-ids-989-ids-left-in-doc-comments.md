---
id: TASK-2647
title: 'ops''s own tree fails ops doc-ids: 989 ids left in doc comments'
status: Done
assignee: []
created_date: '2026-11-01 00:09'
updated_date: '2026-11-01 13:32'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2657'
modified_files:
  - .ops.toml
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/doc_ids_cmd.rs:1`

**What**: `ops doc-ids`, added by this wave, reports 989 task or review-rule ids in `///` and `//!` lines across the workspace, for example `crates/runner/src/command/builtins.rs` and `extensions/tokei/src/views.rs`. The check is therefore not in this repo's `verify` or `qa`.

**Why it matters**: the repo does not pass its own built-in check, and without a gate these ids keep accumulating, which the v0.81-0.82 cleanups already paid for by hand.

**Origin**: discovered during TASK-2645 while fixing TASK-2639.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ops doc-ids exits 0 on this repo
- [x] #2 doc-ids is part of a local gate (verify or qa)

<!-- AC:END -->
