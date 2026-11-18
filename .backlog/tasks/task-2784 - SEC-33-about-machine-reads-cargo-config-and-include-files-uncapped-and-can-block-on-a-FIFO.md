---
id: TASK-2784
title: 'SEC-33: about machine reads cargo config and include files uncapped and can block on a FIFO'
status: To Do
assignee: []
created_date: '2026-11-18 18:44'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2857'
modified_files:
  - extensions/about/src/machine.rs
priority: low
ordinal: 1000
dedup_key: 'SEC-33:extensions/about/src/machine.rs:read_layer'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/machine.rs:313-330` (`read_layer`), reached from `config_layers` (line 172) and `push_with_includes` (line 208)

**What**: `read_layer` uses `std::fs::read_to_string(&path)` with no size cap and no regular-file check. The paths come from the cwd's ancestors' `.cargo/config[.toml]` and, transitively, from each file's `include` array — repo-controlled, and absolute paths are accepted. The crate's own manifest reads go through `ops_core::text::read_capped_to_string`, which caps size and refuses non-regular files precisely so "a FIFO named like a manifest cannot block `ops about`" (manifest_io.rs docs).

**Why it matters**: A checkout whose config includes a FIFO (`include = ["hang.toml"]` with `hang.toml` a named pipe) hangs `ops about machine` indefinitely, and an include pointing at a huge file is read whole into memory. The page is otherwise built to degrade to "unknown" rather than fail.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 read_layer reads through ops_core::text::read_capped_to_string (or an equivalent capped, regular-file-only open) and warns and skips on refusal
- [ ] #2 A unix test with a FIFO-named include returns promptly with that layer skipped
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.22 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
