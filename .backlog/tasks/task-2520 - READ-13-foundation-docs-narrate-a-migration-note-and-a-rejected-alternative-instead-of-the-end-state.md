---
id: TASK-2520
title: 'READ-13: foundation docs narrate a migration note and a rejected alternative instead of the end state'
status: Done
assignee: []
created_date: '2026-10-10 15:33'
updated_date: '2026-10-10 21:42'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2616'
modified_files:
  - extensions-rust/foundation/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/foundation/src/lib.rs:lib'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
<!-- scan confidence: candidates to inspect -->

**File**: `extensions-rust/foundation/src/lib.rs:9` and `extensions-rust/foundation/src/lib.rs:176-179`

**What**: Two doc passages narrate the design journey rather than the behavior (READ-13):

- `extensions-rust/foundation/src/lib.rs:9` — "Updates ship with ops releases; there are no vendored copies to sync." The second clause is a migration note contrasting a past vendored-copies design; a reader using the API gains nothing from it.
- `extensions-rust/foundation/src/lib.rs:176-179` — the `# Errors` paragraph of `workspace_members` closes with "Returning an empty list instead would make check report a clean result it did not establish and scaffold silently add no member opt-ins, so the parse failure propagates." The contract is already stated by the preceding sentence; the counterfactual is a defense of the choice against a rejected alternative, which belongs in the PR description or an ADR.

Surrounding docs were checked and are fine: `check_ops_pin`'s "floor, not an equality" paragraph states an enduring user-visible contract, and `compare`'s module doc explains baseline semantics — both legitimate.

**Why it matters**: Process narration goes stale on the next change while looking authoritative, and it is meaningless to a reader using the API. Rule of thumb: text a new joiner would delete verbatim belongs in the PR description, not the docs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Both passages rewritten to state behavior only, with no counterfactual or migration claims
- [x] #2 A pass over the crate's //! and /// docs confirms no other design-journey narration remains

<!-- AC:END -->
