---
id: TASK-2581
title: 'READ-13: ops-text-fixers eof comments carry rule-ID and task provenance tags'
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
  - extensions/text-fixers/src/eof.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/text-fixers/src/eof.rs:mod eof'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/text-fixers/src/eof.rs:68`, `extensions/text-fixers/src/eof.rs:177`

**What**: Two comments tag themselves with the review rule and backlog task they came from: "// READ-5 / TASK-2253: a run made only of `\r` bytes ..." inside `fix_eof` (line 68), and "/// READ-5 / TASK-2253: a lone trailing CR is a terminator to preserve ..." on the test `lone_trailing_cr_is_preserved_unchanged` (line 177). The technical content (lone-CR terminators are preserved, not converted) is legitimate; the "READ-5 / TASK-2253" prefixes are process artifacts.

**Why it matters**: READ-13 — docs describe the end state, not the journey. Rule-ID and task references are self-report provenance: meaningless to a reader using or modifying the code, and they go stale when tasks close. The project has already stripped this exact pattern from the theme, about, and sqlite crates (recent commits removing task provenance tags from docs and tests).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The 'READ-5 / TASK-2253' prefixes at src/eof.rs:68 and src/eof.rs:177 are removed; the surrounding technical rationale (lone-CR preservation contract) is kept verbatim
- [ ] #2 No TASK-#### or RULE-ID #### tags remain anywhere in extensions/text-fixers/src/eof.rs
<!-- AC:END -->
