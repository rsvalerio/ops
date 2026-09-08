---
id: TASK-2173
title: >-
  READ-13: create-review-tasks-rust docs narrate removed code and TASK history
  instead of the end state
status: To Do
assignee:
  - TASK-2248
created_date: '2026-09-08 07:11'
updated_date: '2026-09-08 11:01'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions-rust/create-review-tasks/src/lib.rs
  - extensions-rust/create-review-tasks/src/provider.rs
priority: medium
ordinal: 86000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/create-review-tasks/src/lib.rs:10-13,50-53`, `extensions-rust/create-review-tasks/src/provider.rs:28-31,64-66,74-82,96-107,109-111,130-141,161-166,225-228,253-255,300-302,338-340,359-360,392-396,420-424`

**What**: Comments and rustdoc across both files are a change journal rather than a description of what the code does now. Representative cases:

- lib.rs:10-13 is a comment whose entire subject is code that no longer exists: "the crate-root `#![cfg_attr(test, allow(..))]` block that used to sit here is gone … the block excused nothing." Nothing in the file relates to it.
- provider.rs:64-66 explains inside `provide` that a previous version wrapped `json!` in `serde_json::to_value` and why that was removed — a diff note, not behaviour.
- provider.rs:74-82 (`root_package_name`) and 96-107 (`member_target_name`) spend most of their doc paragraphs on what the prior implementation did wrong and which task changed it.
- Rule-ID/task-ID prefixes ("ERR-6 / TASK-1812", "DUP-3 / TASK-1814", "PERF-3 / TASK-1819", "PATTERN-1 / TASK-1839", "SEC-11 / TASK-1822", "SEC-14 / TASK-1246") lead ~15 comment and doc blocks, including several test doc comments.

The durable rationale in these blocks is worth keeping — why the member path is guarded before the join, why `?` (Debug) formatting is used for untrusted member strings, why an empty target list is an error. What should go is the narration of prior states and the task-tracker identifiers, which are recoverable from git and the backlog.

**Why it matters**: READ-13. Readers must reconstruct the code's history to learn its current contract, and every reference points at a backlog task rather than at the behaviour. This is the same pattern already filed for other crates (TASK-2101, TASK-2136, TASK-2147, TASK-2155, TASK-2169); this task covers the Rust-stack create-review-tasks crate specifically. The sibling generic crate `extensions/create-review-tasks` is not in scope here.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The lib.rs comment about the removed crate-root cfg_attr allow block is deleted
- [ ] #2 Item and module docs describe the current behaviour and its durable rationale, with no narration of prior implementations or removed code
- [ ] #3 Rule-ID / TASK-NNNN prefixes are removed from comments, rustdoc, and test doc comments in both files; any rationale worth keeping is restated in terms of the behaviour it protects
- [ ] #4 Test doc comments state what the test asserts rather than which backlog task introduced it
<!-- AC:END -->
