---
id: TASK-2681
title: 'FN-1: run_sec_to mixes plan rendering with scan execution (104 lines)'
status: Done
assignee: []
created_date: '2026-11-18 18:27'
updated_date: '2026-11-18 22:07'
labels:
  - code-review-rust
  - FN
dependencies: []
parent_task_id: 'TASK-2843'
modified_files:
  - crates/cli/src/sec_cmd.rs
priority: low
ordinal: 1000
dedup_key: 'FN-1:crates/cli/src/sec_cmd.rs:run_sec_to'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/sec_cmd.rs:743` (`run_sec_to`)

**What**: `run_sec_to` is 104 lines and works at three abstraction levels in one body: override validation and scan-root resolution, the full `--dry-run` preview (plan, scan root, skip dirs, ignore file, dev deps, outside files, trivy probe), and the live run (empty-plan refusal, trivy probe, outside notice, scan loop and aggregation). Siblings in the crate (`check_lock` 89 lines, `run_init_to` 89 lines) are long too, but this is the largest production function in the crate.

**Why it matters**: `ops sec` is the fail-closed terminal step of `ops qa`; the exit-code logic is interleaved with presentation, which makes the fail-closed branches harder to review and test in isolation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Dry-run preview rendering is extracted into its own function (e.g. write_dry_run(w, &plan, &opts, ...))
- [x] #2 Live execution and exit-code aggregation is extracted into its own function
- [x] #3 run_sec_to stays under 50 lines with existing sec_cmd tests unchanged and passing

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.02 (code-review-rust run, 2026-11-18).

run_sec_to is now 29 lines: SecRun::prepare resolves plan/opts/outside, write_dry_run renders the preview, run_live runs the scans and aggregates the exit code.

<!-- SECTION:NOTES:END -->
