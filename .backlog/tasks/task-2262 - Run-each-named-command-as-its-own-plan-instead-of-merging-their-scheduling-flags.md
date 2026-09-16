---
id: TASK-2262
title: 'Run each named command as its own plan instead of merging their scheduling flags'
status: To Do
assignee: []
created_date: '2026-09-16 17:01'
updated_date: '2026-09-16 17:12'
labels:
  - bug
  - runner
dependencies: []
parent_task_id: 'TASK-2265'
modified_files:
  - crates/cli/src/run_cmd/plan.rs
  - crates/cli/src/run_cmd/mod.rs
  - crates/cli/src/run_cmd/tests.rs
priority: high
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`ops verify qax` merges both commands into one plan in `merge_plan` (`crates/cli/src/run_cmd/plan.rs:23`). `any_parallel` is ORed across every name, so as soon as one command is parallel (the rust-stack `verify` has been since 0.56.0), the whole merged plan runs in parallel, including the steps of `qa`/`qax`, which declare `parallel = false`.

This contradicts the README ("Naming several commands on one invocation (`ops run verify qa`) expands each independently, so they may differ").

Observed in dbsec with `ops verify qax`: `deps`, `next`, `test-doc` and `sec` all ran at the same time as each other and as verify's check stage. `sec` (Trivy) walked `target/` while the trybuild test inside `next` was creating and deleting files there, and it failed:

    fs scan error: ... stat .../target/tests/trybuild/.../.fingerprint/dbsec-derive-tests-.../output-bin-trybuild031: no such file or directory

`ops sec` alone passes. The workaround is `ops verify && ops qax`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Naming several commands runs them one after another, each scheduled with its own `parallel`, `fail_fast` and `exclusive` settings
- [ ] #2 `ops verify qax` with a parallel `verify` and a sequential `qax` runs verify's stages, then the qax steps one at a time
- [ ] #3 Under fail_fast, a failing command stops the commands named after it
- [ ] #4 The run still shows a single progress display and summary covering every step
- [ ] #5 A test covers a parallel name followed by a sequential name
<!-- AC:END -->
