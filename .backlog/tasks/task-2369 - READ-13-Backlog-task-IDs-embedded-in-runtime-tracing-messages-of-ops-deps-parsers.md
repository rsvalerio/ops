---
id: TASK-2369
title: 'READ-13: Backlog task IDs embedded in runtime tracing messages of ops-deps parsers'
status: Triage
assignee: []
created_date: '2026-10-04 14:11'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions-rust/deps/src/parse/deny.rs
  - extensions-rust/deps/src/parse/upgrade.rs
  - extensions-rust/deps/src/format.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/deps/src/parse:tracing-messages'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse/deny.rs`, `extensions-rust/deps/src/parse/upgrade.rs`, `extensions-rust/deps/src/format.rs`

<!-- scan confidence: candidates to inspect -->

**What**: Operator-visible `tracing` message text is prefixed with internal backlog IDs or rule IDs that mean nothing to a user reading logs, and describe the journey (which ticket added the guard) rather than the event. Candidates (non-test): `format.rs:356` (TASK-0602), `parse/upgrade.rs:108` (TASK-1074), `:149` (TASK-1202), `:171` (TASK-2179), `:291` (TASK-1026), `:343` and `:348` (TASK-0404), `:389` (TASK-0960), `parse/deny.rs:239`, `:351`, `:361`, `:373` (TASK-1840), `:329` (`ERR-1:`), `:383` (TASK-0845), `:439` (TASK-0597), `:488` (TASK-0436).

**Why it matters**: Log text is user-facing output; ticket IDs are process artifacts that go stale and add noise to filtering/grouping. Tests asserting on these strings (if any) must be updated with the messages.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No tracing message in ops-deps non-test code starts with a TASK-NNNN or rule-ID prefix; messages describe the event only
- [ ] #2 Existing tests that match on the old message text are updated and pass
<!-- AC:END -->
