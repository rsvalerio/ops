---
id: TASK-2559
title: 'READ-13: config-checkers error docs cite code-review rule IDs instead of stating the behavior'
status: Done
assignee: []
created_date: '2026-10-10 15:40'
updated_date: '2026-10-10 22:12'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2624'
modified_files:
  - extensions/config-checkers/src/error.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/config-checkers/src/error.rs:CheckError'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/error.rs:10`

**What**: The production docs and comments in error.rs cite code-review rulebook IDs rather than describing the property in substance: the public doc on `LimitExceeded` says the input 'is willing to spend (`SEC-33`)' (error.rs:10), and the comment in the `Display` impl for `CheckError::Parse` explains the delegation choice as avoiding 'the `ERR-9` duplication' (error.rs:62). A third instance sits in test docs (json.rs:236, `SEC-33 regression:`) — scoped out as test code, listed for completeness.

**Why it matters**: Rule-ID self-references are process artifacts — meaningless to a reader using the API, and they go stale while looking authoritative. READ-13 flags exactly this 'self-report of which guidelines the code follows' shape; sibling precedent TASK-2223 (about-java, same rule, LOW). The prior READ-13 cleanup for this crate (TASK-2391) removed the change-history narration but these citations survived. Each citation has real content one sentence away: SEC-33 is 'a resource-amplification bound the checker imposes so tiny inputs cannot exhaust memory', and ERR-9 is 'rendering the source in the message would duplicate it for chain-walking printers'.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 No code-review rule IDs (SEC-33, ERR-9, etc.) remain in production docs or comments of extensions/config-checkers/src
- [x] #2 Each rewritten comment states the property it guards in substance (resource bound, message/source duplication) without referencing the review rulebook

<!-- AC:END -->
