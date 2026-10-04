---
id: TASK-2353
title: 'DUP-1: atomic_write and atomic_write_noclobber duplicate the staging sequence'
status: Triage
assignee: []
created_date: '2026-10-04 14:08'
labels:
  - code-review-rust
  - DUP
dependencies: []
modified_files:
  - crates/backlog/src/cmd/mod.rs
priority: low
ordinal: 1000
dedup_key: 'DUP-1:crates/backlog/src/cmd/mod.rs:atomic_write'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/cmd/mod.rs:95-187` (`atomic_write`, `atomic_write_noclobber`)

**What**: The two functions share an identical ~25-line prefix and body: file-name check, staging-name construction (`.{name}.{pid}.tmp`), `create_new` open, `write_all` + `sync_all` with cleanup on error. They differ only in the final publish step (`rename` vs `hard_link` plus the AlreadyExists mapping and staging removal).

**Why it matters**: A fix to staging (for example a unique-suffix change, fsync of the parent dir, or error text) must be made twice and can drift, in code that guards task-file durability.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Staging (name, create_new open, write, sync, cleanup on error) lives in one private helper used by both functions
- [ ] #2 Existing atomic_write and noclobber tests pass unchanged
<!-- AC:END -->
