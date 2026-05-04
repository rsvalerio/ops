---
id: TASK-0970
title: 'PERF-3: cargo-update strip_ansi allocates a fresh String per stderr line'
status: Triage
assignee: []
created_date: '2026-05-04 21:48'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:163-183, 90-127`

**What**: `parse_update_output` calls `strip_ansi(trimmed)` for every line of cargo's stderr, allocating a new String of `s.len()` capacity per line — even when no ANSI sequence is present. A noisy `cargo update` produces hundreds of lines.

**Why it matters**: Hot path on the data-source pipeline used by CI. Avoidable allocations per line.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Fast path: scan once; if no \x1b is present, parse trimmed directly without allocation
- [ ] #2 Or return Cow<'_, str> from strip_ansi
- [ ] #3 Microbench/trace confirms reduction
<!-- AC:END -->
