---
id: TASK-1178
title: >-
  PATTERN-1: ProgressState::record_stderr does two HashMap probes on cold-id
  path
status: To Do
assignee:
  - TASK-1270
created_date: '2026-05-08 08:09'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display/progress_state.rs:129`

**What**: The bookkeeping does `if let Some(buf) = self.step_stderr.get_mut(id) { buf } else { self.step_stderr.entry(id.to_string()).or_default() }`. The first arm avoids `id.to_string()` on hits but the second arm always allocates. A single `entry` lookup keyed by a Cow-friendly approach would dedupe the two probes, matching the PATTERN-1 / TASK-0998 fix already applied in `merge_alias_for`.

**Why it matters**: Two hash probes per stderr line in the cold-id case, plus inconsistent shape with the Entry-routed cleanup elsewhere in the crate. Pattern drift invites the same bug TASK-0998 cleaned up.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The two branches collapse onto a single entry lookup so each call probes once.
- [ ] #2 Behavior parity test on hit / miss / cap-eviction unchanged.
<!-- AC:END -->
