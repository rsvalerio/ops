---
id: TASK-2572
title: 'READ-13: git_state.rs docs open with rule-ID self-report prefixes'
status: To Do
assignee: []
created_date: '2026-10-10 15:43'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - read
dependencies: []
parent_task_id: 'TASK-2624'
modified_files:
  - extensions/hook-common/src/git_state.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/hook-common/src/git_state.rs:git_state module docs'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/git_state.rs:11,15,45,81,127,133,150,307`

**What**: Eight `///` docs open with review-rule self-report prefixes: '/// ASYNC-6: grace period...' (11, 15, 45), '/// ERR-1: bounded wait...' (81), '/// CONC-3: stdout is routed...' (127), '/// ERR-5: the stderr drain thread...' (133), '/// READ-4: .stderr(Stdio::piped())...' (150), and the test doc at 307. These narrate which review guideline the change followed rather than what the code does — a process artifact. The underlying doc prose (bounded-wait contract, single-shot-process caveat, stderr pipe invariant) is end-state documentation and stays.

**Why it matters**: READ-13 — self-reports of which guidelines a change followed are meaningless to a reader using the API and go stale on the next change while looking authoritative. The crate-level cleanup applied to crates/theme in e5b5443f (strip tag, keep prose) has not reached this file.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No doc comment in the file opens with a rule-ID prefix; the contract prose under each doc is kept
- [ ] #2 cargo test -p ops-hook-common passes
<!-- AC:END -->
