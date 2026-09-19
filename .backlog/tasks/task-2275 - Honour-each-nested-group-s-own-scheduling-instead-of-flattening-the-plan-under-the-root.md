---
id: TASK-2275
title: 'Honour each nested group''s own scheduling instead of flattening the plan under the root'
status: Triage
assignee: []
created_date: '2026-09-19 11:23'
labels:
  - feature
  - runner
dependencies: []
modified_files:
  - crates/runner/src/command/resolve.rs
  - crates/cli/src/run_cmd/plan.rs
  - README.md
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A composite that nests others is flattened into one plan and scheduled by its root (`expand_to_leaves_with_flags`, `crates/runner/src/command/resolve.rs`). The README says so: "Expressing run these groups in order, but let the steps inside one group run together is not supported today; it needs per-group scheduling boundaries."

Consequence in dbsec (ops 0.60.0): `pre-release = ["verify", "deps", "test-doc", "sec", "release-tests", "deny", "forge-sync", "cog-check"]` with `parallel = false` runs `verify`'s 14 steps one at a time -- the staged parallel schedule it gets from `ops verify` is lost. And a parallel root cannot contain a sequential group at all (the conflicting-`parallel` load error), so there is no way to write the reverse either.

TASK-2262 already runs each *command-line name* as its own plan, one after another, with its own `parallel`/`fail_fast`/`exclusive`. The same treatment for a sequential root's direct children would cover the common case: a sequential group whose entries are groups runs each child as its own plan, in order, stopping under `fail_fast` -- exactly what `ops verify deps ...` does from the shell. `pre-release` would then mean what typing its list means.

Open questions: whether a parallel root may contain a sequential child (run it as one exclusive stage?); how `fail_fast` disagreements between a parent and child resolve once they no longer share a plan; whether hooks (`run-before-commit`) change behaviour.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A sequential group whose entries include groups runs each child group with that child's own parallel/exclusive schedule, one child after another
- [ ] #2 Under fail_fast a failing child stops the children after it
- [ ] #3 `ops <seq-group>` and `ops <child1> <child2> ...` produce the same schedule and results
- [ ] #4 Nested parallel-inside-sequential no longer runs sequentially; a test pins the staged schedule of a parallel child under a sequential root
- [ ] #5 The README "not supported today" note is replaced by the new rule, including what a parallel root with a sequential child does
- [ ] #6 Still one progress display and summary for the whole run
<!-- AC:END -->
