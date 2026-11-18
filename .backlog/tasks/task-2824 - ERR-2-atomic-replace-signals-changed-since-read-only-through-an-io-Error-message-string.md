---
id: TASK-2824
title: 'ERR-2: atomic::replace signals ''changed since read'' only through an io::Error message string'
status: To Do
assignee: []
created_date: '2026-11-18 18:49'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - ERR
dependencies: []
parent_task_id: 'TASK-2859'
modified_files:
  - extensions/text-fixers/src/atomic.rs
  - extensions/text-fixers/src/runner.rs
priority: low
ordinal: 1000
dedup_key: 'ERR-2:extensions/text-fixers/src/atomic.rs:replace'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/text-fixers/src/atomic.rs:77-90,119-121,145-151` (`replace`, `CHANGED_SINCE_READ`, `ensure_unchanged`)

**What**: The public `atomic::replace` documents one of its failure modes as "An error whose message is [`CHANGED_SINCE_READ`]" and produces it with `io::Error::other(CHANGED_SINCE_READ)` (kind `Other`). The only way for a caller to tell "refused because the file changed under us" apart from any other `Other` I/O error is to compare `err.to_string()` against a public string constant — the tests do exactly that (`atomic.rs:247,265,291,308`, `runner.rs:697`). The symlinked-directory refusal similarly reuses `ErrorKind::InvalidInput`, a kind ordinary I/O can also return.

**Why it matters**: The concurrent-edit refusal is a distinct, expected outcome a caller may want to treat differently (e.g. retry, or report as "skipped: edited" rather than a write failure). Discriminating by message text is fragile — any rewording or wrapping (`with_context`, `io::Error::new(kind, format!(..))`) silently breaks it — and the variant set is not visible in the type.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 replace returns a typed error (or an io::Error wrapping a typed inner error retrievable via get_ref/downcast) with distinct variants for changed-since-read and symlinked-directory refusals
- [ ] #2 Tests and runner identify the changed-since-read case by variant, not by comparing the message string
- [ ] #3 replace's # Errors section names each variant
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.31 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
