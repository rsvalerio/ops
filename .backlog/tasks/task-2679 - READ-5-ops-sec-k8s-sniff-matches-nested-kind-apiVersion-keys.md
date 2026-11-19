---
id: TASK-2679
title: 'READ-5: ops sec k8s sniff matches nested kind/apiVersion keys'
status: Done
assignee: []
created_date: '2026-11-18 18:26'
updated_date: '2026-11-18 22:07'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2843'
modified_files:
  - crates/cli/src/sec_cmd.rs
priority: low
ordinal: 1000
dedup_key: 'READ-5:crates/cli/src/sec_cmd.rs:is_k8s_manifest'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/sec_cmd.rs:234` (`is_k8s_manifest`), comment at `:248`, code at `:250`

**What**: The comment states the invariant "Top-level mapping keys are unindented; `trim_start` also tolerates a leading document marker". The code does the opposite on both counts: `line.trim_start()` strips indentation, so any nested `kind:` / `apiVersion:` key (common in CI configs, OpenAPI specs, Helm values, GitHub workflow files) counts as a top-level match; and `trim_start` only strips whitespace, so it does not handle a `---` document marker at all.

**Why it matters**: Any repo with a YAML file carrying nested `kind:` and `apiVersion:` keys gets the misconfiguration scan auto-selected, and `--dry-run` reports "Dockerfile, Kubernetes, or IaC files detected" when none exist. The documented invariant and the behaviour disagree, so the next editor cannot tell which one is intended.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Only unindented apiVersion:/kind: keys (optionally after a '---' marker line) count as a manifest signature
- [x] #2 The comment describes the implemented behaviour
- [x] #3 A test covers a YAML file with nested kind:/apiVersion: that is not detected as a manifest

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.02 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
