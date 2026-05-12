---
id: TASK-1359
title: >-
  FN-1: extension_summary mixes type-flag classification, command-name
  collection, and self-shadow audit/dedup tracking
status: Done
assignee:
  - TASK-1383
created_date: '2026-05-12 21:29'
updated_date: '2026-05-12 23:16'
labels:
  - code-review-rust
  - complexity
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/extension_cmd.rs:155`

**What**: `extension_summary` is ~45 lines and bundles three concerns: (1) type-flag classification into `Vec<String>`, (2) two-branch command-name collection (static accessor vs runtime `register_commands` probe), (3) self-shadow duplicate audit with `tracing::warn!` + `warned: &mut HashSet<...>` bookkeeping. Callers that only want the `(Vec<String>, Vec<String>)` summary must thread a dummy `warned` set.

**Why it matters**: The audit's side-effecting warn + dedupe-set is orthogonal to the summary's return contract and forces every caller to participate in the audit policy. Split into `extension_summary` (pure, no `warned` parameter) and a sibling `audit_command_self_shadow(ext, &mut warned)` invoked at call sites that want the audit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Split extension_summary into a pure summary fn and a separate audit helper; remove the &mut warned parameter from the summary's signature
- [ ] #2 All existing list/show callers compile; new audit helper has a dedicated unit test; warning behavior unchanged
<!-- AC:END -->
