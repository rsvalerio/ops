---
id: TASK-2485
title: 'READ-13: tests.rs comments carry TASK-id provenance and journey narration'
status: To Do
assignee: []
created_date: '2026-10-10 15:30'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2615'
modified_files:
  - extensions-rust/cargo-update/src/tests.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/cargo-update/src/tests.rs:tests'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/tests.rs:5`

**What**: Two sites narrate provenance instead of the end state:
- lines 5-8 (module doc): "The tracing-capture harness (BufWriter + MakeWriter + the global-dispatcher pin) and the control-character assertion come from the shared ops-about test-support module **rather than a local re-implementation that can drift**." The "rather than a local re-implementation" clause is how-the-change-was-made narration.
- line 1319: `/// TASK-2298: modern cargo appends (available: vX) ...` — a TASK-id tag prefix on an otherwise legitimate behavioral rationale.

**Why it matters**: READ-13 — the rule of thumb is that text a newcomer would delete verbatim belongs in the PR description, not the docs. The repo stripped this same class from ops-git (TASK-2394), ops-theme, and ops-about; cargo-update's test module still carries it. The enduring facts (the harness comes from ops-about's test-support; cargo appends the `(available: vX)` annotation) survive without the task tags and the before/after framing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No comment in tests.rs references a TASK id
- [ ] #2 The module doc states where the shared harness lives without the 'rather than a local re-implementation' journey clause
<!-- AC:END -->
