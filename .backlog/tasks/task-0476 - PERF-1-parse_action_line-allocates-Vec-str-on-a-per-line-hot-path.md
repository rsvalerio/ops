---
id: TASK-0476
title: 'PERF-1: parse_action_line allocates Vec<&str> on a per-line hot path'
status: Done
assignee:
  - TASK-0533
created_date: '2026-04-28 05:47'
updated_date: '2026-04-28 17:57'
labels:
  - code-review-rust
  - PERF
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:194-231`\n\n**What**: For each parsed line, parse_action_line allocates a Vec<&str> (twice on the Update branch). Cargo-update output is small but the function runs per stderr line during provide() inside a must_use data-provider path used by metadata pipelines in CI.\n\n**Why it matters**: PERF-1 / PATTERN-3: prefer iterator destructuring (let mut it = rest.splitn(4, ' '); let name = it.next()?; ...) to avoid intermediate Vec.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Replace Vec<&str> = rest.splitn(...).collect() with iterator-based destructuring
- [x] #2 Tests still pass without behaviour change
<!-- AC:END -->
