---
id: TASK-2519
title: 'READ-13: tests/subprocess.rs docs carry TASK provenance tags'
status: To Do
assignee: []
created_date: '2026-10-10 15:33'
updated_date: '2026-10-10 21:12'
labels:
  - code-review
  - read
dependencies: []
parent_task_id: 'TASK-2617'
modified_files:
  - extensions-rust/test-coverage/src/tests/subprocess.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/test-coverage/src/tests/subprocess.rs:mod subprocess'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/tests/subprocess.rs:10-12,51,70-71,102,124-126,172-177,210-212,242-251`

**What**: Test docs throughout open with rule/TASK tags: "TASK-1595", "PATTERN-1 / TASK-1099" (three times, narrating the pre-fix error format), "TEST-23 / TASK-1554" with the include_str!-rot essay, the "CONC-9 / TASK-2068" banner comment block, and the "AC #1 + AC #2, the finding itself" doc which narrates the pre-fix 15-minute stall ("Pre-fix the wait was a fixed 15 minutes regardless, so an operator who set ...").

**Why it matters**: READ-13: docs that say "the finding itself" and "Pre-fix ..." are review-process narration, not test documentation. The enduring content is scenario + expected outcome, already encoded in the test names.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No test doc in tests/subprocess.rs opens with a RULE-ID / TASK-XXXX tag, and none narrate the pre-fix behaviour or reference 'the finding'
<!-- AC:END -->
