---
id: TASK-1605
title: >-
  READ-2: probe timeout warn message embeds rule IDs and task IDs in
  operator-facing log line
status: Done
assignee:
  - TASK-1637
created_date: '2026-05-22 06:43'
updated_date: '2026-05-22 12:52'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe/timeout.rs:64`

**What**: The `tracing::warn!` emitted when a probe times out carries the literal message
`"ASYNC-6 / TASK-0914 + API / TASK-1200: probe timed out; reporting tool as ProbeFailed"`.
Internal rule/task identifiers leak into a log line that operators see when `cargo --list`
or `rustup component list` hangs in their environment.

TASK-1313 covered the analogous sweep for source comments. Comments are tolerable; log
messages render to humans diagnosing a real failure and the rule/task IDs are noise to
them (they cannot look the IDs up).

**Why it matters**: Log messages are operator-facing UX. Embedding internal task IDs:
- adds noise to the actionable signal (`probe timed out; reporting tool as ProbeFailed`)
- rots over time as task IDs are renumbered or archived
- invites operators to file confused issues referencing IDs from a private backlog
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Log message in  Timeout branch no longer contains  or rule-ID prefixes (, , etc.)
- [x] #2 Wording still conveys the operationally-relevant facts: which probe (), that it timed out, that the tool will be reported as ProbeFailed
- [x] #3 If rule/task provenance is desired, move it to a code comment above the  call, not into the message string
<!-- AC:END -->
