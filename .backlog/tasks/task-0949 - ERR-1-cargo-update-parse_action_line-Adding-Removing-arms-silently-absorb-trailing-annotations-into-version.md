---
id: TASK-0949
title: >-
  ERR-1: cargo-update parse_action_line Adding/Removing arms silently absorb
  trailing annotations into version
status: Triage
assignee: []
created_date: '2026-05-04 21:33'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:252`

**What**: `parse_action_line` for `Adding`/`Removing` lines uses `rest.split_once(' ')`, capturing everything after the name as the version. A future cargo annotation (e.g. `Adding new-crate v0.1.0 (locked)`) is silently glued onto `version_raw`. TASK-0613 closed only the `Updating` arm.

**Why it matters**: A format-drift annotation goes unnoticed and corrupts the `from`/`to` field with extra tokens, producing wrong-but-plausible update entries — the same regression class TASK-0613 fixed for `Updating`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 For Adding/Removing, validate that rest contains exactly <name> <version> (no trailing tokens) and warn loudly when extra tokens are present, mirroring the Updating arm's it.next().is_some() check
- [ ] #2 Add a unit test asserting a line like 'Adding new-crate v0.1.0 (locked)' either parses with version 0.1.0 and emits a warn, or returns None — never produces version '0.1.0 (locked)'
<!-- AC:END -->
