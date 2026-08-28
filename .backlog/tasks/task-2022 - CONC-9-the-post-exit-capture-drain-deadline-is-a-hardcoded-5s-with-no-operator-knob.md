---
id: TASK-2022
title: >-
  CONC-9: the post-exit capture drain deadline is a hardcoded 5s with no
  operator knob
status: Triage
assignee: []
created_date: '2026-08-28 19:42'
labels:
  - code-review-rust
  - concurrency
dependencies: []
modified_files:
  - crates/runner/src/command/exec.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs` (`POST_EXIT_DRAIN_GRACE`, `POST_KILL_DRAIN_GRACE`, `spawn_capped`)

**What**: TASK-1919 bounded the post-exit pipe drain: after `child.wait()` returns, `spawn_capped` waits at most `POST_EXIT_DRAIN_GRACE` (5s) for both readers, then `SIGKILL`s the process group to release the inherited pipe write end, then waits a further `POST_KILL_DRAIN_GRACE` (2s) before failing the step with a `TimedOut` error. That converts a permanent hang into a bounded one, which is the point — but both values are compile-time constants, unlike every other resource knob in this crate (`OPS_OUTPUT_BYTE_CAP`, `OPS_MAX_PARALLEL`, `OPS_PARALLEL_EVENT_BUDGET`), which are env-overridable with validation and a warn-on-fallback.

Two consequences worth a decision:

1. A step that *deliberately* leaves a descendant streaming to the inherited pipe after the leader exits now has its output truncated at 5s and the descendant killed. That is a behaviour change relative to "hang forever", and strictly better, but it is not tunable by an operator whose workload legitimately needs longer.
2. On a heavily loaded CI runner the 5s could in principle be reached by a slow-but-progressing drain, converting a healthy step into a killed one.

Options: expose `OPS_OUTPUT_DRAIN_GRACE_SECS` through the same `parse`-and-warn shape the other knobs use, or document the constants as deliberately fixed with the reasoning.

**Why it matters**: it is the one resource deadline in the exec path an operator cannot see or change, and it can now terminate a descendant process — the highest-consequence default in the module.

**Origin**: discovered during TASK-1986 while fixing TASK-1919.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 the post-exit drain deadline is either env-overridable with the crate's standard parse/clamp/warn shape, or documented as deliberately fixed with the reasoning
- [ ] #2 if a knob is added, an out-of-range or unparseable value falls back to the default with a tracing::warn, matching OPS_OUTPUT_BYTE_CAP
<!-- AC:END -->
