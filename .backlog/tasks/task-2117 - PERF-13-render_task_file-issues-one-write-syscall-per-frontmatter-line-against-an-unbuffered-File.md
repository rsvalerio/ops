---
id: TASK-2117
title: >-
  PERF-13: render_task_file issues one write syscall per frontmatter line
  against an unbuffered File
status: To Do
assignee:
  - TASK-2244
created_date: '2026-09-08 06:54'
updated_date: '2026-09-08 10:58'
labels:
  - code-review-rust
  - performance
dependencies: []
modified_files:
  - extensions/create-review-tasks/src/backlog.rs
  - extensions/create-review-tasks/src/lib.rs
priority: low
ordinal: 35000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/create-review-tasks/src/backlog.rs:149` (`render_task_file`), called from `extensions/create-review-tasks/src/lib.rs:409` (`stage_task_file`)

**What**: `stage_task_file` hands the raw `std::fs::File` returned by `File::create_new` straight to `render_task_file`, which then emits the document with ~14 separate `writeln!` calls. `std::fs::File` performs no buffering, so each `writeln!` is its own `write(2)`. The doc comment on `render_task_file` cites PERF-13 for writing "straight into `w`; no intermediate `String` per line" — which avoids the allocations but lands on the more expensive of the two failure modes, a syscall per line.

**Why it matters**: Every task file costs an order of magnitude more syscalls than it needs, and the cost is multiplied by the retry loop: a run that loses the allocation race re-renders the whole set on each of up to `MAX_ALLOCATION_ATTEMPTS` attempts. It also makes partial-file states more granular if a write fails mid-document. Wrapping the handle in a `BufWriter` at the `stage_task_file` call site keeps the "no intermediate String" property while collapsing the document into one write — but note that a `BufWriter` must be flushed explicitly (its `Drop` discards errors), so the flush has to be checked and its error mapped to the same `writing {path}` context the current code uses.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Task-file rendering goes through a buffered writer so the document reaches the filesystem in one write rather than one per line
- [ ] #2 The buffer is flushed explicitly and a flush failure is propagated with the same path-naming context as the current write errors, not swallowed by BufWriter's Drop
- [ ] #3 The render_task_file doc comment no longer claims a PERF property the call site does not deliver
<!-- AC:END -->
