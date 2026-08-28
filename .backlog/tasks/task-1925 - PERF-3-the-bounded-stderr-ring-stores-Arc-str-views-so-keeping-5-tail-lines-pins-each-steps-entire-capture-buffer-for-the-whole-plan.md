---
id: TASK-1925
title: >-
  PERF-3: the bounded stderr ring stores Arc<str> views, so keeping 5 tail lines
  pins each step's entire capture buffer for the whole plan
status: To Do
assignee:
  - TASK-1986
created_date: '2026-08-27 15:45'
updated_date: '2026-08-28 14:10'
labels:
  - code-review-rust
  - performance
dependencies: []
modified_files:
  - crates/runner/src/display/progress_state.rs
  - crates/runner/src/display/render_config.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display/progress_state.rs:125-143` (`record_stderr`), `crates/runner/src/display/progress_state.rs:27-30` (the ring's memory claim), `crates/runner/src/display.rs:549-555` (`on_step_output`), `crates/runner/src/display/render_config.rs:382-391` (`StderrTail::cap`)

**What**: `ProgressState`'s doc comment states the ring's purpose:

> `step_stderr` is a bounded ring per id sized by the caller-supplied cap in `record_stderr`. PERF-1 / TASK-0539: prior implementation held every captured stderr line for the plan's lifetime even though only the configured tail (`stderr_tail_lines`, default 5) is ever rendered.

Two things break that claim now that the ring holds `OutputLine` instead of `String`:

**1. Each retained line pins the whole buffer.** `OutputLine` is `{ buf: Arc<str>, range: Range<usize> }` (`command/events.rs:509-513`) — a view onto the *entire* per-stream capture buffer that `exec_command` wraps once (`exec.rs:461-465`). Holding one line keeps the `Arc<str>` alive, so a step that wrote 4 MiB of stderr keeps all 4 MiB resident for as long as any of its five tail lines sits in the ring — i.e. until the next `reset_for_plan`. Across a plan the display retains one full capture buffer per step that produced stderr, up to `steps × OPS_OUTPUT_BYTE_CAP` (default 4 MiB), which is the *same* "every captured line for the plan's lifetime" order of magnitude TASK-0539 set out to remove. The ring bounds the deque's element count, not the bytes it keeps alive. This is the classic substring-retains-parent-allocation shape, and the ring's own regression test (`record_stderr_bounded_ring_keeps_only_tail`) cannot see it because it feeds `format!("line {i}").into()`, which allocates a fresh one-line `Arc<str>` per call — a shape production never produces.

**2. Verbose mode removes the bound entirely.** `--verbose` selects `StderrTail::Unbounded` (`display.rs:176-180`), whose `cap()` returns `usize::MAX`, so `record_stderr` never evicts and the deque grows one `OutputLine` (24 bytes) per stderr line for every step, on top of the pinned buffers. The existing test `record_stderr_high_cap_preserves_full_tail` pins this as intended behaviour, so it is a deliberate trade — but it is undocumented at the `StderrTail::Unbounded` definition and is the mode CI runs in most often.

Fix direction: since the tail is at most `max_lines` short lines and `extract_stderr_tail` (`display/error_detail.rs:429-434`) stringifies them anyway on the failure path, copying to `String` (or a small `Box<str>`) at `record_stderr` time costs one allocation per *retained* line and releases the megabyte-scale buffer as soon as the step's events have been processed.

**Why it matters**: this is display-side retention on top of the runner-side retention, both scaling with plan length, both invisible to the `OPS_MAX_PARALLEL` knob the docs point operators at. It also makes the module's stated PERF-1 guarantee untrue, which is how the next reviewer gets misled.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 record_stderr no longer keeps a step's full capture buffer alive via a retained OutputLine view — retained tail lines own only their own bytes
- [ ] #2 the ProgressState doc comment's O(tail) claim is true as written, or is corrected to describe actual retention
- [ ] #3 a regression test feeds many lines that all share one Arc<str> buffer (as production does) and asserts the buffer is released once the ring has evicted past it — e.g. via Arc::strong_count on the shared buffer
- [ ] #4 StderrTail::Unbounded documents that --verbose intentionally removes the eviction bound, and the resulting growth is bounded or acknowledged
<!-- AC:END -->
