---
id: TASK-0347
title: 'SEC-21: tap option in dry-run path may surface raw env values via vars.expand'
status: To Do
assignee:
  - TASK-0419
created_date: '2026-04-26 09:34'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd/dry_run.rs:69-76`

**What**: print_exec_spec correctly redacts when key looks sensitive or expanded value matches the heuristic, but uses vars.expand(v) first and then checks looks_like_secret_value_public(&expanded). If the heuristic misses, the expanded value is printed verbatim.

**Why it matters**: Defense-in-depth note (SEC-21 — no secrets in printed output). dry-run is exactly when users redirect output to bug-report pastes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add a dry-run test asserting that an env entry whose name matches a custom user-supplied sensitive prefix list is redacted regardless of value heuristics
- [ ] #2 Document the heuristic known false-negatives in a SEC-21 comment near the redaction site
<!-- AC:END -->
