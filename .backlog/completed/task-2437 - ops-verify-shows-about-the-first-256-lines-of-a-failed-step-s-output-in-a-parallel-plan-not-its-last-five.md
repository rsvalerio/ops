---
id: TASK-2437
title: 'ops verify shows about the first 256 lines of a failed step''s output in a parallel plan, not its last five'
status: Done
assignee: []
created_date: '2026-10-09 19:38'
updated_date: '2026-10-09 20:28'
labels:
  - runner
  - bug
dependencies: []
modified_files:
  - crates/runner/src/command/exec.rs
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:863` (`exec_standalone`, `LOCAL_BUF = 256`)

**What**: in a parallel plan, a step's `StepOutput` events go through a 256-slot local channel with `try_send`. A step that prints more than about 256 lines overflows it and the later lines are dropped, with the count reported only at debug level. CONC-7 already routes the terminal event around the buffer for this reason, but the output lines themselves are still lost. The failure box's "stderr (last 5 lines)" is therefore roughly lines 250 to 260 of the step's output, not its end. A sequential plan shows the real last lines.

**Why it matters**: the failure box is what a gate runner and a CI reader see first. When it shows lines from the middle of the output, the real error is absent. In dbsec one nextest failure went unnamed twice (dbsec TASK-1353, TASK-1362) and was taken for an unidentifiable flaky test. dbsec works around it for its `next` step only, by passing `--status-level leak` so a run stays under the buffer (dbsec `.ops.toml`, `[extend.next]`); any other step that prints more than about 256 lines before failing still shows the wrong lines.

**Reproduction** (dbsec, 2026-10-09, ops 0.79.0; the code is unchanged on ops main at v0.79.1): add a deliberately failing test late in a large nextest run (test 775 of 1026) and run `ops verify`. The box shows five PASS lines at 245 to 249 of 1026. `ops next` alone, a sequential plan, shows the summary and the FAIL line.

**Origin**: reported from dbsec TASK-1370.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A failed step in a parallel plan shows the real last lines of its stderr, however many lines it printed
- [x] #2 Dropped output lines, if any remain possible, are reported at a level a user sees without debug logging

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in PR #89 (a81dd145): exec_standalone awaits each output line on the runner channel instead of try_send into a 256-slot buffer. Lines abandoned under fail-fast abort or a closed receiver are counted and reported at the default log level.
<!-- SECTION:NOTES:END -->
