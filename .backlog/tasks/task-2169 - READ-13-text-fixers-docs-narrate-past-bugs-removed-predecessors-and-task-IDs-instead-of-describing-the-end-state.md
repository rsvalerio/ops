---
id: TASK-2169
title: >-
  READ-13: text-fixers docs narrate past bugs, removed predecessors and task IDs
  instead of describing the end state
status: To Do
assignee:
  - TASK-2248
created_date: '2026-09-08 07:07'
updated_date: '2026-09-08 11:01'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions/text-fixers/src/lib.rs
  - extensions/text-fixers/src/trailing.rs
  - extensions/text-fixers/src/binary.rs
  - extensions/text-fixers/src/runner.rs
  - extensions/text-fixers/src/discovery.rs
  - extensions/text-fixers/src/report.rs
priority: low
ordinal: 82000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/text-fixers/src/lib.rs:20-27,50-58`, `extensions/text-fixers/src/trailing.rs:20-27`, `extensions/text-fixers/src/binary.rs:60-63`, `extensions/text-fixers/src/atomic.rs:180-184` (doc on `record_failure` at `runner.rs:180-185`), `extensions/text-fixers/src/discovery.rs:33-38,145-155,186-190`, `extensions/text-fixers/src/report.rs:160-165`

**What**: item, module and crate docs are written as a change log rather than a
description of the code as it stands:

- `lib.rs:20-27` — "Previously an unreadable file was skipped with no message,
  no counter and no effect on the exit code, so a mode-600 file … made the run
  report a clean tree it had never looked at."
- `lib.rs:50-58` — a whole block explaining which four lints a *removed*
  `cfg_attr(test, allow(..))` used to relax, keyed to `READ-10 (TASK-1966)`.
- `trailing.rs:20-27` — "It previously did not. `has_crlf` was computed from
  the byte before `line_end`, and in the no-newline-found branch …", including
  the old buggy behaviour for two specific inputs.
- `binary.rs:60-63` (test doc) — "The predecessor of this function inspected
  only the first 8 KiB and called this buffer text; `run_fixer` then rewrote
  it."
- `runner.rs:180-185` — "The predecessor propagated a bare `io::Error`, so a
  repository-wide run could fail with `Permission denied (os error 13)` …"
- `discovery.rs:33-38` — "Before this was centralised the two modes disagreed
  by accident: `walk` dropped symlinks only because …"
- `discovery.rs:186-190`, `report.rs:160-165`, `runner.rs` — several more
  "used to"/"before this" paragraphs, plus `discovery.rs:145-155` citing
  `SEC-33 / TASK-2052` by task number.

**Why it matters**: READ-13 — documentation describes the end state, not the
journey. A reader of `trailing.rs` today has to parse two paragraphs about a
bug that no longer exists to find the one sentence that states the current
invariant, and `lib.rs`'s lint block documents an attribute that is not in the
file. Backlog task IDs in doc comments (`TASK-1966`, `TASK-2052`) date the
source against a tracker the reader may not have. The rationale worth keeping
(what the code guarantees and why the trade was made) is present and good; it
is the retrospective framing around it that should go — the history already
lives in git.

Same finding as TASK-2136 in `ops-run-before-commit`; resolve them the same
way.

<!-- scan confidence: verified — every listed site read in full -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Docs state what the code guarantees now; 'previously', 'used to', 'the predecessor' and 'before this' framing is removed
- [ ] #2 The lib.rs block describing the removed cfg_attr(test, allow(..)) is deleted
- [ ] #3 Backlog task IDs (TASK-1966, TASK-2052) are removed from doc comments; the reasoning they carry is kept in its own right where it is still load-bearing
- [ ] #4 The design rationale that justifies current trade-offs (symlink policy, whole-buffer binary sniff, rename-based writes, per-file failure policy) survives the edit
<!-- AC:END -->
