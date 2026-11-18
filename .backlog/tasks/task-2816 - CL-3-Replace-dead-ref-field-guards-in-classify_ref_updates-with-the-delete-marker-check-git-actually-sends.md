---
id: TASK-2816
title: 'CL-3: Replace dead ref-field guards in classify_ref_updates with the (delete) marker check git actually sends'
status: To Do
assignee: []
created_date: '2026-11-18 18:47'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - CL
dependencies: []
parent_task_id: 'TASK-2862'
modified_files:
  - extensions/run-before-push/src/lib.rs
  - extensions/run-before-push/tests/classify_ref_updates.rs
priority: low
ordinal: 1000
dedup_key: 'CL-3:extensions/run-before-push/src/lib.rs:classify_ref_updates'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-push/src/lib.rs:237-249` (`classify_ref_updates`)

**What**: After `line.split_whitespace()` the four fields can never be empty, so `local_ref.is_empty() || remote_ref.is_empty()` is unreachable — it reads as ref validation but validates nothing. Meanwhile the one property that decides a skip is left implicit: a line is counted as a deletion purely because its local oid is all zeros, without checking that git's paired marker `local_ref == "(delete)"` is present. git always sends both together, so a zero local oid with any other local ref is input the classifier did not "fully understand", yet it is treated as a deletion and can yield `PushRefs::DeleteOnly` (which skips every configured check).

**Why it matters**: The function's own contract says it "only ever *skips* work on input it fully understood". The dead guard hides that the deletion precondition is an undocumented assumption about git's stream rather than a checked one; the fail-open direction here is a skipped verification gate. Dead branches also produce surviving mutants.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The unreachable is_empty() checks on local_ref/remote_ref are removed
- [ ] #2 A line counts as a deletion only when local_ref is "(delete)" AND local oid is all zeros; a mismatch between the two returns PushRefs::Run
- [ ] #3 Tests cover a zero local oid with a non-(delete) local ref, and a (delete) local ref with a non-zero oid, both classifying as Run
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.29 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
