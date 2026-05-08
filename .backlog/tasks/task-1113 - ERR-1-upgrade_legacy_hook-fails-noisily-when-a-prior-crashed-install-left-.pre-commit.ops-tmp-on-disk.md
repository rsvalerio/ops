---
id: TASK-1113
title: >-
  ERR-1: upgrade_legacy_hook fails noisily when a prior crashed install left
  .pre-commit.ops-tmp on disk
status: Done
assignee: []
created_date: '2026-05-07 21:50'
updated_date: '2026-05-08 06:17'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/install.rs:124`

**What**: `upgrade_legacy_hook` builds a fixed temp path `parent.join(format!(".{file_name}.ops-tmp"))` and `write_temp_hook` opens it via `OpenOptions::new().create_new(true)`. A prior crash between `write_temp_hook` and the rename leaves the temp file on disk; the next `ops install` invocation then fails at `write_temp_hook` with `AlreadyExists`, the `inspect_err` arm removes the leftover, but the *current* install still propagates the error to the operator. The user must rerun `ops install` to recover.

**Why it matters**: Hook upgrade is meant to be idempotent and self-healing. A single transient crash silently turns into a one-shot failed install with a confusing `AlreadyExists` error that does not name the temp file. Operators chasing "ops install fails on this repo" have no breadcrumb and no documented workaround. A fix is to (a) detect a stale temp on `AlreadyExists`, remove it, and retry once, or (b) use a randomized/`tempfile`-style sibling name so leftovers cannot collide with the current install.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 upgrade_legacy_hook recovers from a leftover .ops-tmp sibling without surfacing AlreadyExists to the caller, or the failure mode is documented and emits a tracing::warn naming the leftover path
<!-- AC:END -->
