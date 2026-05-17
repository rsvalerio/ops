---
id: TASK-1473
title: >-
  PERF-3: subprocess::read_capped post-cap discard loop should use io::copy into
  io::sink
status: Done
assignee:
  - TASK-1479
created_date: '2026-05-16 10:06'
updated_date: '2026-05-17 07:42'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/subprocess.rs:240-265`

**What**: After reaching the cap, the read loop keeps read-ing into `chunk` and discarding — necessary to keep the child unblocked — but uses an unbounded `loop { reader.read(&mut chunk) }` rather than `io::copy(&mut reader, &mut io::sink())` for the discard path, and unconditionally re-checks `remaining` per iteration.

**Why it matters**: When the child produces gigabytes past the cap (an adversarial or runaway tool), the drain thread spins per-8KiB rather than dispatching to the kernel splice-style copy. Memory bound is preserved, but the comment claims "amortised" — it isn't.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Once buf.len() == cap, switch to io::copy(&mut reader, &mut io::sink()) and track its byte count instead of looping byte-wise in chunk
- [ ] #2 Add a benchmark or rough timing assertion that draining a 100 MiB over-cap stream completes in under the timeout without 8 KiB-step CPU dominance
<!-- AC:END -->
