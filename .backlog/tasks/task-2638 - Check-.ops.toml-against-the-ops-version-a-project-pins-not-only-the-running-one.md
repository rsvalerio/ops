---
id: TASK-2638
title: 'Check .ops.toml against the ops version a project pins, not only the running one'
status: To Do
assignee: []
created_date: '2026-10-26 22:12'
updated_date: '2026-10-31 23:55'
labels:
  - feature
  - config
  - ci
dependencies: []
parent_task_id: 'TASK-2646'
modified_files: []
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
CI installs ops from mise.toml's pin, while developers often run a newer ops built from source in ~/.cargo/bin, which shadows the mise shim. A .ops.toml field newer than the pin passes every local gate and fails every ops step in CI.

It happened twice in dbsec: raw = true (added in 0.85.0) failed CI on the 0.79.3 pin, and args beside clone (0.86.0) failed on 0.85.1 (TOML parse error). dbsec now carries scripts/ops-pin-check.sh (100 lines) as a pre-push hook: it installs the pinned ops through mise and dry-runs the composites with it.

ops knows when each config field arrived. Proposal: record the release each field/behaviour was introduced in, and add ops config check [--as <version>] (defaulting to the mise.toml ops pin when present) that names each field the target version would reject, plus the minimum version that accepts the file. ops foundation, which already compares the pin as a floor, could run it too. Note that parsing alone is not enough: args beside clone used to be a load error, but a behaviour change that still parses would pass silently.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ops config check --as 0.85.1 on a file using args beside clone names that field and 0.86.0
- [ ] #2 Without --as it uses mise.toml's ops pin when there is one
- [ ] #3 The minimum accepting version is printed
<!-- AC:END -->
