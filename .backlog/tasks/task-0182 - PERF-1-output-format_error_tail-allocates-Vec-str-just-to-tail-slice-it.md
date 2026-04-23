---
id: TASK-0182
title: 'PERF-1: output::format_error_tail allocates Vec<&str> just to tail-slice it'
status: Done
assignee: []
created_date: '2026-04-22 21:25'
updated_date: '2026-04-23 15:16'
labels:
  - rust-code-review
  - PERF
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: crates/core/src/output.rs:20-24

**What**: format_error_tail does let lines: Vec<&str> = stderr_str.lines().collect(); tail_lines(&lines, n).join("\n"). For a 10 000-line stderr buffer this collects every line into a Vec only to drop all but the last n. The str::lines() iterator is fused and cheap; you can count once, then skip to len - n with .skip(count.saturating_sub(n)) (iterating twice) or more allocation-friendly, collect into a small ring buffer of size n using an arrayvec / heapless or just a VecDeque<&str> with a capacity cap. For typical n = 5 this is a trivial saving; it matters on stacks that dump large build logs (e.g., failed cargo test runs).

**Why it matters**: PERF-1 / PATTERN-3. Not a hot path today, but the shape is the canonical "premature .collect()" anti-pattern called out in the rules. Left as-is, the cost scales with the upstream buffer size instead of n.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Avoid collecting the entire Vec<&str>; keep only the last n lines (VecDeque with bounded capacity, or two-pass over the iterator)
- [ ] #2 Benchmark on a multi-MB stderr to confirm the saving
<!-- AC:END -->
