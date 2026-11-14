---
id: TASK-2352
title: 'READ-13: Rustdoc in ops-extension narrates change history and task IDs'
status: Done
assignee: []
created_date: '2026-10-04 14:08'
updated_date: '2026-10-04 15:52'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2423'
modified_files:
  - crates/extension/src/data.rs
  - crates/extension/src/extension.rs
  - crates/extension/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:crates/extension/src:module docs'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs:223-419` (DataRegistry::register, provider_names, provide, struct docs), `crates/extension/src/extension.rs:18-276` (CommandRegistry, EXTENSION_REGISTRY, insert), `crates/extension/src/lib.rs:3-19`

**What**: `///`, `//!` and `//` blocks describe the journey, not the end state: "previously the implementation called HashMap::insert", "the previous implementation also fired a debug_assert!(false)", "Previously `register` returned ()", "previously paired with a provider_names_iter method", and rule-ID / TASK-NNNN tags on most items (data.rs ~24 occurrences, extension.rs ~14). The `register` doc is ~60 lines, mostly change history. lib.rs carries a verification log ("Verified: cargo check ... clean").

**Why it matters**: Readers of the public API get migration notes instead of the contract, and the text goes stale while looking authoritative. History belongs in commit messages / backlog tasks.

<!-- scan confidence: candidates to inspect -->
Candidates: data.rs:4,23,76,211,223,241,263,271,284,305-314,338,357,391-419,439,467,488,505; extension.rs:18,37,57,109,153,165,186,202-206,223,231,236,270,276; lib.rs:3-19; error.rs (1), macros.rs (2).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Docs on public items state the current contract only, with no 'previously'/'the previous implementation' narration
- [x] #2 Rule-ID / TASK-NNNN provenance tags removed from rustdoc and comments, unless the comment explains a non-obvious current invariant
- [x] #3 lib.rs verification-log comment trimmed to the enduring rationale for #![forbid(unsafe_code)]

<!-- AC:END -->
