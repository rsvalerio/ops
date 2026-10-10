---
id: TASK-2492
title: 'READ-13: sec_cmd module docs narrate TASK provenance instead of the end state'
status: To Do
assignee: []
created_date: '2026-10-10 15:30'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2611'
modified_files:
  - crates/cli/src/sec_cmd.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:crates/cli/src/sec_cmd.rs:module docs'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/sec_cmd.rs:23,30,53,73,86,278`

**What**: The module and item doc comments carry `TASK-####` provenance tags and change-history narration ("TASK-2264: the single shared list is...", "used to print nothing and exit 0", "rather than a blanket `**/build` (TASK-2271)"). The durable rationale (why build dirs are skipped, why the gate fails closed) is good — the journey narration around it is the READ-13 violation.

**Why it matters**: Process artifacts in docs go stale on the next change while looking authoritative; the repo is actively stripping these (recent commits cleaned theme/about/sqlite the same way). Sites: 23 (module doc), 30 (module doc heading), 53 (module doc), 73 (shared_skip_dirs), 86 (shared_skip_dirs), 278 (WalkOutcome).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No /// or //! line in sec_cmd.rs references a TASK id or narrates how a change came to be
- [ ] #2 The durable why (skip policy, fail-closed exit code) is kept as end-state prose
<!-- AC:END -->
