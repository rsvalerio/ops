---
id: TASK-2454
title: 'READ-13: style.rs test docs carry RULE/TASK provenance tags'
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
  - crates/theme/src/style.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:crates/theme/src/style.rs:mod tests'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/style.rs:36`, `:60`, `:122`, `:136`, `:151`, `:175`

**What**: Doc comments on tests in the inline `mod tests` open with provenance tags: `READ-5/TASK-0355` (:36), `PERF-3 / TASK-0746` (:60), `SEC-11 / TASK-1967 AC#2 + CL-3 / TASK-2019` (:122, a // comment inside the proptest), `SEC-11 / TASK-1967 AC#1` (:136), `SEC-11 / TASK-1967 AC#2` (:151), `CL-3 / TASK-1969` (:175).

**Why it matters**: READ-13 — guideline self-report tags are process artifacts. The behavioural contracts they prefix (OSC-8 stripping, width-agreement invariant, control-byte elimination, truncation policy) are worth keeping; the tag prefixes are not. Leftover from before the task-2351.06-era docs cleanup that already swept most of the crate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No comment in style.rs references a TASK id or rule-ID prefix; the documented invariants remain
<!-- AC:END -->
