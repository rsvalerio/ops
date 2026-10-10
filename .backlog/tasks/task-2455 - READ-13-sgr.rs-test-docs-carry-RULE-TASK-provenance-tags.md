---
id: TASK-2455
title: 'READ-13: sgr.rs test docs carry RULE/TASK provenance tags'
status: To Do
assignee: []
created_date: '2026-10-10 15:24'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2609'
modified_files:
  - crates/theme/src/style/sgr.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:crates/theme/src/style/sgr.rs:mod tests'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/style/sgr.rs:197`, `:213-216`

**What**: Doc comments on tests in the inline `mod tests` open with provenance tags: `CL-3 / TASK-1976: the theme renders to stderr only...` (:197) and `DUP-3 / TASK-1188: both color subsystems must agree...` plus a follow-up `...TASK-1976). Pinning equivalence...` (:213-216).

**Why it matters**: READ-13 — same class as the style.rs tags: guideline self-report prefixes on otherwise durable behavioural documentation (stderr-only gating; cross-crate colour-subsystem equivalence).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No comment in sgr.rs references a TASK id or rule-ID prefix; the documented gate/equivalence contracts remain
<!-- AC:END -->
