---
id: TASK-2638
title: 'Check .ops.toml against the ops version a project pins, not only the running one'
status: Done
assignee: []
created_date: '2026-10-26 22:12'
updated_date: '2026-11-01 00:41'
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
- [x] #1 ops config check --as 0.85.1 on a file using args beside clone names that field and 0.86.0
- [x] #2 Without --as it uses mise.toml's ops pin when there is one
- [x] #3 The minimum accepting version is printed

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented as crates/core/src/config/history.rs (release table, raw-file walk, mise pin) and crates/cli/src/config_cmd.rs (ops config check [--as]). ops init --rust --check and ops foundation report the same rejected fields as waivable drift. Field releases were computed by scanning every release tag's config structs.

Next-unreleased convention: a new field goes under the next minor after the workspace version. after and wrap (wave 75) landed first, so this wave covered them under 0.91.0 on rebase; wave 76 added no config field. every_config_field_is_in_the_history fails until a new field is listed.

[themes.*.error_block] has no rows: it ignores unknown keys, so no release rejects a field inside it. Value and behaviour tracking: TASK-2650. Other mise pin locations: TASK-2651. Pin parsing duplicate: TASK-2652.
<!-- SECTION:NOTES:END -->
