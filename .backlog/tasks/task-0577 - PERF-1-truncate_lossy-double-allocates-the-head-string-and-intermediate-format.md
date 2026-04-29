---
id: TASK-0577
title: >-
  PERF-1: truncate_lossy double-allocates the head string and intermediate
  format!
status: Triage
assignee: []
created_date: '2026-04-29 05:17'
labels:
  - code-review-rust
  - PERF
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/results.rs:114`

**What**: truncate_lossy decodes `bytes[..cap]` via `String::from_utf8_lossy`, calls `.into_owned()`, then `head.push_str(&format!("[ops] output truncated: ..."))` — the `format!` allocates a transient String only to be immediately copied. For large outputs (default cap 4 MiB per stream) the prefix allocation is unavoidable, but the marker line allocation is gratuitous.

**Why it matters**: PERF-1. Function is on per-step hot path. Use `write!(head, "...")` via std::fmt::Write to skip the intermediate allocation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Marker append uses write!(&mut head, ...) without intermediate format! allocation
- [ ] #2 Existing tests pass
<!-- AC:END -->
