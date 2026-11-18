---
id: TASK-2773
title: 'DUP-5: config-checkers discovery reporting drifts from the fixers'' copy and drops undecodable_paths'
status: Done
assignee: []
created_date: '2026-11-18 18:44'
updated_date: '2026-11-18 21:37'
labels:
  - code-review-rust
  - duplication
dependencies: []
parent_task_id: 'TASK-2859'
modified_files:
  - extensions/config-checkers/src/runner.rs
  - extensions/text-fixers/src/runner.rs
priority: medium
ordinal: 1000
dedup_key: 'DUP-5:extensions/config-checkers/src/runner.rs:discover_candidates'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/runner.rs:141` (`discover_candidates`), counterpart `extensions/text-fixers/src/runner.rs:146` (`report_discovery_issues`)

**What**: `discover_candidates` re-implements the fixers' `report_discovery_issues` (fallback notice + `report_walk_errors`) by hand, and the copy has already drifted: it never reads `Discovery::undecodable_paths`. `ops_text_fixers::discovery` documents (discovery.rs:60-68, 121-123) that on non-Unix targets a tracked path that cannot be decoded is dropped from `files` and counted there "so the caller reports it rather than swallowing it". The fixers print `N tracked path(s) skipped: filename is not valid UTF-8 on this platform`; the checkers print nothing, count nothing in `files_skipped`, and the summary line reports a clean tree.

**Why it matters**: On Windows, a tracked `*.json`/`*.yaml` with an undecodable name is silently never validated, and `check-json --tracked` exits 0 — the same fail-open class the walk-error handling in this crate exists to close. The root cause is two hand-maintained copies of one notice sequence; the next field added to `Discovery` will drift the same way.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The config checkers surface Discovery::undecodable_paths (a notice on the writer and a count in the report, e.g. folded into files_skipped or its own field) instead of dropping it
- [x] #2 The discovery-notice sequence (fallback, walk errors, undecodable paths) lives in one shared helper used by both the fixers and the checkers, or a test pins that both consumers handle every Discovery field
- [x] #3 A test (cfg-gated or via a constructed Discovery) proves a non-zero undecodable count is reported by the checkers

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.23 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
