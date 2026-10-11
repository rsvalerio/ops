---
id: TASK-2538
title: 'READ-13: ops-deps tests.rs docs carry TASK-id provenance tags'
status: Done
assignee: []
created_date: '2026-10-10 15:36'
updated_date: '2026-10-10 21:45'
labels:
  - code-review
  - read
dependencies: []
parent_task_id: 'TASK-2618'
modified_files:
  - extensions-rust/deps/src/tests.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/deps/src/tests.rs:mod tests'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/tests.rs:115`, `extensions-rust/deps/src/tests.rs:416`, `extensions-rust/deps/src/tests.rs:442`

**What**: Test docs open with backlog task provenance: `/// TASK-2324: ops deps --check needs neither cargo-edit...` on `run_deps_check_needs_no_cargo_edit_and_bypasses_the_provider`, `/// TASK-2326: the reported tools are the probed ones...`, and `/// SEC-13 / TASK-2336: the registered command spawns an absolute...`. The leading tag narrates which change added the test instead of what behaviour it pins.

**Why it matters**: Task IDs are process artifacts that go stale while looking authoritative (READ-13: docs describe the end state, not the journey). Waves 44-51 removed the identical pattern from other crates' test docs (e.g. task-2514 for test-coverage's tests/parse.rs); these three predate or escaped that cleanup in ops-deps.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 No TASK-number token remains anywhere in extensions-rust/deps/src/tests.rs
- [x] #2 Each cleaned test doc still states the behaviour under test without the task reference

<!-- AC:END -->
