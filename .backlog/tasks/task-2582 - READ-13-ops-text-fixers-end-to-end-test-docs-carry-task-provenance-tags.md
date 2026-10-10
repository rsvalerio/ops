---
id: TASK-2582
title: 'READ-13: ops-text-fixers end-to-end test docs carry task provenance tags'
status: To Do
assignee: []
created_date: '2026-10-10 15:45'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - read
dependencies: []
parent_task_id: 'TASK-2627'
modified_files:
  - extensions/text-fixers/src/tests.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/text-fixers/src/tests.rs:mod tests'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/text-fixers/src/tests.rs:71`, `extensions/text-fixers/src/tests.rs:536`, `extensions/text-fixers/src/tests.rs:562`

**What**: Three test doc comments open with backlog-task provenance: "/// TASK-2322: check mode reports ..." (line 71), "/// SEC-13 / TASK-2122: the registered fixers spawn an absolute ..." (line 536), and "/// TASK-2322: each fixer has a registered `--check` twin ..." (line 562). The behavioral descriptions that follow are good test documentation; the task-number prefixes are process artifacts.

**Why it matters**: READ-13 — documentation describes the end state, not the journey that produced it. Task tags in test docs are history narration: a reader verifying the check-mode contract or the exclusivity contract does not need the task number that introduced it, and the tags go stale as tasks close. The project has already stripped this exact pattern from the theme, about, and sqlite crates (recent commits removing task provenance tags from docs and tests).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The TASK-2322 / SEC-13 / TASK-2122 prefixes at src/tests.rs:71, :536 and :562 are removed; the behavioral descriptions they introduced are kept
- [ ] #2 No TASK-#### tags remain anywhere in extensions/text-fixers/src/tests.rs
<!-- AC:END -->
