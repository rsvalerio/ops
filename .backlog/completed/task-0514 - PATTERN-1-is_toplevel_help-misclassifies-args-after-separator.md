---
id: TASK-0514
title: 'PATTERN-1: is_toplevel_help misclassifies args after ''--'' separator'
status: Done
assignee:
  - TASK-0536
created_date: '2026-04-28 06:51'
updated_date: '2026-04-28 16:19'
labels:
  - code-review-rust
  - correctness
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/help.rs:13`

**What**: is_toplevel_help treats `--` as just another flag (starts with `-`); `ops -- --help` returns true and prints top-level help, dropping a subcommand the user might have expected to receive --help verbatim.

**Why it matters**: `--` is a clap-recognised end-of-options marker; treating it transparently breaks pass-through semantics for external/dynamic commands.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Stop scanning when -- is seen
- [x] #2 Test asserts ops -- --help is not classified top-level
<!-- AC:END -->
