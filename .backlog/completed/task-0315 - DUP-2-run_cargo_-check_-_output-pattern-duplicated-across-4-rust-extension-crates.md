---
id: TASK-0315
title: >-
  DUP-2: run_cargo_* + check_*_output pattern duplicated across 4 rust extension
  crates
status: Done
assignee:
  - TASK-0326
created_date: '2026-04-24 08:53'
updated_date: '2026-04-25 13:14'
labels:
  - rust-code-review
  - duplication
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: extensions-rust/metadata/src/lib.rs:30-48; extensions-rust/test-coverage/src/lib.rs:92-114; similar in cargo-update and deps

**What**: cargo-update, metadata, test-coverage, deps all repeat: build Command, run_with_timeout, format_error_tail, bail.

**Why it matters**: DUP-2 cross-crate duplication; a retry/timeout change needs to touch four call sites.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract shared helper (in crates/core/subprocess or new extensions-rust/common) taking argv + timeout + label
- [ ] #2 All four callsites refactored to the helper; tests pass
<!-- AC:END -->
