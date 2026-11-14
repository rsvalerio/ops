---
id: TASK-2391
title: 'READ-13: Docs and comments narrate change history instead of the end state'
status: Done
assignee: []
created_date: '2026-10-04 14:15'
updated_date: '2026-10-04 15:59'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2422'
modified_files:
  - extensions/config-checkers/src/yaml.rs
  - extensions/config-checkers/src/tests.rs
  - extensions/config-checkers/src/runner.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/config-checkers/src/yaml.rs:module-docs'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/yaml.rs:21-24`, `extensions/config-checkers/src/tests.rs:293-296`, `extensions/config-checkers/src/runner.rs:123-127`

**What**: Doc and comment text describes the journey rather than the current behaviour. `yaml.rs` module doc: "Parser errors are reported exactly as before ... input past MAX_NESTING_DEPTH or MAX_EXPANDED_NODES now fails where the loader path accepted it" (and the `check_yaml` loader-vs-event comparison). `tests.rs` walk_errors doc: "Before this, the error was printed to the writer and dropped". `tests.rs` device-symlink test: "`discovery` now applies that type test...". `runner.rs`: "Sharing it is what picked up the symlink hardening ... the divergence this dedup existed to close". Many comments also carry `TASK-NNNN` ids as the main justification (lib.rs, report.rs, runner.rs, yaml.rs).

**Why it matters**: "as before", "now", "before this" are meaningless to a reader who joined later and go stale; the migration story belongs in the PR/commit/ADR. The enduring facts (event-level parsing bounds memory; limits are stream-wide) should stay.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Docs/comments state current behaviour without 'as before', 'now', 'before this' or dedup/migration narrative
- [x] #2 Retained rationale (event-level parse, stream-wide budget, walk-error fail-closed) is kept as timeless statements

<!-- AC:END -->
