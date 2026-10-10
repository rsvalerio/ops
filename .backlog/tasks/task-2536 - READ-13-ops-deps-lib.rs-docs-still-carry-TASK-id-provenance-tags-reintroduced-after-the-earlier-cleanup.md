---
id: TASK-2536
title: 'READ-13: ops-deps lib.rs docs still carry TASK-id provenance tags reintroduced after the earlier cleanup'
status: To Do
assignee: []
created_date: '2026-10-10 15:36'
updated_date: '2026-10-10 21:12'
labels:
  - code-review
  - read
dependencies: []
parent_task_id: 'TASK-2618'
modified_files:
  - extensions-rust/deps/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/deps/src/lib.rs:crate-root-docs'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/lib.rs:10`, `extensions-rust/deps/src/lib.rs:247`, `extensions-rust/deps/src/lib.rs:543`

**What**: Crate docs and inline comments cite backlog task IDs as provenance: `(TASK-2324)` in the crate-level `//!` docs for `ops deps --check`, `(TASK-2326)` on the `ExternalTool` doc, and `// SEC-13 / TASK-2336:` in the `register_commands` closure. TASK-2182 already cleaned the rustdoc change-journal from this file; these three tags reference later tasks and reintroduce the same pattern.

**Why it matters**: Task IDs are process artifacts meaningless to a reader using the API; they go stale while looking authoritative (READ-13: docs describe the end state, not the journey that produced it). Past waves removed the identical pattern from ops-metadata, ops-about-terraform, run-before-commit, and the test-coverage parsers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No TASK-number token remains anywhere in extensions-rust/deps/src/lib.rs
- [ ] #2 Each cleaned doc still states the behaviour it documents (the --check contract, the ExternalTool source, the PATH-shim rationale) without the task reference
<!-- AC:END -->
