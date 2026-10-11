---
id: TASK-2468
title: 'PATTERN-1: SearchOptions keeps a dead plain bool beside json instead of the OutputFormat enum'
status: Done
assignee: []
created_date: '2026-10-10 15:27'
updated_date: '2026-10-10 21:34'
labels:
  - code-review-rust
  - pattern
dependencies: []
parent_task_id: 'TASK-2613'
modified_files:
  - crates/backlog/src/cmd/search.rs
priority: low
ordinal: 1000
dedup_key: 'PATTERN-1:crates/backlog/src/cmd/search.rs:SearchOptions'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/cmd/search.rs:23` (struct SearchOptions), read site `crates/backlog/src/cmd/search.rs:52` (run_search)

**What**: `SearchOptions` carries both `plain: bool` and `json: bool`. `run_search` branches on `opts.json` only — `opts.plain` is never read anywhere in the crate (the CLI crate sets `plain: plain || !json`, crates/cli/src/backlog_cmd.rs:362, to no effect). `plain: true, json: true` is representable and silently renders JSON: the exact invalid state the crate's own `OutputFormat` doc (crates/backlog/src/cmd/mod.rs:37-50) says the enum was introduced to make unrepresentable, and which `ViewOptions`/`ListOptions` already use (`format: OutputFormat`).

**Why it matters**: A public options field that does nothing misleads every caller into thinking `--plain` is honored in search; and the two-bool pair reintroduces the modelling flaw the crate documents as fixed elsewhere. Fix: replace both fields with `format: OutputFormat` and update the CLI construction site.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 SearchOptions has no dead output-mode bool; the output mode is selected by an OutputFormat field
- [x] #2 No caller-visible behaviour change: search --plain and --json render exactly as before

<!-- AC:END -->
