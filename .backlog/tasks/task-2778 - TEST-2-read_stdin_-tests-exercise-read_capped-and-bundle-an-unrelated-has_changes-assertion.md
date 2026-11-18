---
id: TASK-2778
title: 'TEST-2: read_stdin_* tests exercise read_capped and bundle an unrelated has_changes assertion'
status: Done
assignee: []
created_date: '2026-11-18 18:44'
updated_date: '2026-11-18 22:57'
labels:
  - code-review-rust
  - TEST
dependencies: []
parent_task_id: 'TASK-2856'
modified_files:
  - extensions-terraform/plan/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-2:extensions-terraform/plan/src/lib.rs:tests'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:1132` (`tests::read_stdin_rejects_oversized_payload`), `lib.rs:1437` (`tests::read_stdin_at_cap_returns_payload`)

**What**: Both tests are named for `read_stdin`, but they call `read_capped` on a `Cursor` with the `"on stdin"` label, so `read_stdin` itself (the `stdin().lock()` wiring and the `env.json_cap` it receives) is never exercised. `read_stdin_at_cap_returns_payload` also ends with `assert!(!has_changes(&[]))`, which is unrelated to its scenario and duplicates `has_changes_false_when_empty`. `read_stdin_rejects_oversized_payload` is otherwise the same as `read_capped_rejects_oversized_terraform_show_output` apart from the source label (TEST-12).

**Why it matters**: TEST-2 asks for one concern per test, with a name that states the scenario. A name that promises stdin coverage it does not deliver misleads whoever audits the cap's ingress points. The tacked-on assertion means a `has_changes` regression would fail a test named for stdin reading.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Tests named for read_stdin either exercise the stdin path or are renamed to describe read_capped at-cap / over-cap behaviour
- [x] #2 The has_changes(&[]) assertion is removed from read_stdin_at_cap_returns_payload
- [x] #3 The over-cap read_capped tests that differ only in their source label are consolidated (e.g. one table-driven test)

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.21 (code-review-rust run, 2026-11-18).

AC #2 was already satisfied on the landing branch: read_stdin_at_cap_returns_payload no longer carried the has_changes(&[]) assertion. Renamed the at-cap test to read_capped_at_cap_returns_payload_unchanged and folded the over-cap stdin/show tests into one table-driven read_capped test.

<!-- SECTION:NOTES:END -->
