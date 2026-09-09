---
id: TASK-2099
title: 'READ-13: design-journal doc essays and the itemless registry_duplicate_policy doc-anchor module'
status: To Do
assignee: []
created_date: '2026-09-08 06:38'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - structure
dependencies: []
parent_task_id: 'TASK-2248'
modified_files:
  - crates/extension/src/lib.rs
  - crates/extension/src/data.rs
  - crates/extension/src/error.rs
priority: low
ordinal: 24000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/lib.rs:21-40`, `crates/extension/src/data.rs:12-44`, `crates/extension/src/data.rs:233-259`, `crates/extension/src/data.rs:633-650`, `crates/extension/src/error.rs:163-187`

**What**: READ-13 flags docs that narrate *how a change was designed* rather than the end state. Extremes in this crate:

- `pub mod registry_duplicate_policy {}` (lib.rs:21-40) — an **empty public module** that exists only to host a doc comment; it ships in the public API surface with zero items.
- `DEFAULT_PROVIDER_BUDGET` (data.rs:12-44) — a 32-line essay narrating TASK-2056/TASK-2068 history and why 20 minutes.
- `DataProvider::provide` trait docs (data.rs:233-259) — a "Decision: it stays synchronous" design journal with both rejected alternatives.
- `DuckDbHandle` docs (data.rs:585-650) — the downcast hazard contract (keep) plus a rejected-alternatives essay.
- `ComputationFailed` docs (error.rs:163-187) — a re-derivation table of display paths (ERR-9/TASK-1889).

**Why it matters**: READ-13's rule of thumb: text a new joiner would delete verbatim belongs in the PR description or an ADR, not the docs. These essays go stale on the next change while looking authoritative, and every doc comment taxes every future reader of an otherwise small crate. The *enduring* content (the policy table, the reborrow-first downcast contract, the plain-`{}`-loses-the-chain rendering rule) should stay; the task history and the rejected-alternatives narration should go. Filed Low because this is deliberate house style with self-documented rationale.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each cited doc block reduced to the enduring contract: the invariant, the required caller behaviour, and the consequence of misuse; TASK-N provenance and rejected-alternatives essays removed (moved to ADR/PR history if not already in the backlog task)
- [ ] #2 The itemless `pub mod registry_duplicate_policy` is gone: its policy table lives on the two methods it governs (DataRegistry::register, CommandRegistry::insert) or a named static, so no empty public module remains in the API
- [ ] #3 rustdoc intra-doc links still resolve (workspace denies broken links); public API shape unchanged except the removed empty module
<!-- AC:END -->
