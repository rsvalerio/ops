---
id: TASK-2628
title: 'READ-13: residual TASK provenance tags in crates/cli main, test utils, registry and theme/about sites'
status: Triage
assignee: []
created_date: '2026-10-10 21:34'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - crates/cli/src/main.rs
  - crates/cli/src/test_utils.rs
  - crates/cli/src/pre_hook_cmd.rs
  - crates/cli/src/registry/registration.rs
  - crates/cli/src/registry/discovery.rs
  - crates/cli/src/registry/tests.rs
  - crates/cli/src/hook_shared.rs
  - crates/cli/src/about_cmd.rs
  - crates/cli/src/theme_cmd.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: multiple — see the per-file list below.

**What**: After wave TASK-2611 cleaned the sec_cmd / run_cmd / args / help / scattered member sites, crates/cli still carries TASK-tag provenance and history narration at:
- `crates/cli/src/main.rs`:223,236,279,379
- `crates/cli/src/test_utils.rs`:6,68,96,102
- `crates/cli/src/pre_hook_cmd.rs`:30 (SEC-11 / TASK-1906 in the PUSH_OPS gate comment)
- `crates/cli/src/registry/registration.rs`:269
- `crates/cli/src/registry/discovery.rs`:41,56,230
- `crates/cli/src/registry/tests.rs`:373
- `crates/cli/src/hook_shared.rs`:183,192,335,388
- `crates/cli/src/about_cmd.rs`:281
- `crates/cli/src/theme_cmd.rs`:683

These are outside every TASK-2611 member's acceptance criteria (the nearest tasks name other files), so they were left rather than fixed out of scope.

**Why it matters**: READ-13 — docs must describe the end state; TASK/RULE tags and fix-history narration are process artifacts that rot as the code moves on.

**Origin**: discovered during TASK-2611 (code-review-plan-wave57) while fixing TASK-2492/2493/2498/2499/2500.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every listed site describes the end state in present-tense prose; TASK/RULE provenance tags are gone (backlog-domain TASK-id examples excepted)
- [ ] #2 No behavioural change: docs/comments only, ops verify clean
<!-- AC:END -->
