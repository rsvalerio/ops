---
id: TASK-1422
title: 'PERF-3: emit_to writes one writeln! syscall per line of a multi-line message'
status: To Do
assignee:
  - TASK-1458
created_date: '2026-05-13 18:18'
updated_date: '2026-05-13 19:09'
labels:
  - code-review-rust
  - perf
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/ui.rs:50`

**What**: `emit_to` iterates `message.split('\n')` and calls `writeln!(w, ...)` per line against an unbuffered locked stderr. A multi-line anyhow chain (common for the `load_config_or_default` failure path) issues N stderr syscalls. Stderr is conventionally line-buffered in some terminals but unbuffered when piped to a non-terminal (the typical CI/capture case).

**Why it matters**: Minor under interactive use but per-line syscalls multiply in CI where stderr is captured. Build the full rendered output into a single `String` (one allocation, known size) and emit with one `write_all`. Also reduces interleaving risk with parallel writers to stderr.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 render full output to a single buffer and write once to the writer
- [ ] #2 continuation-line indentation and SEC-21 sanitisation behaviour unchanged
- [ ] #3 existing ui::warn / ui::error tests pin output byte-for-byte
<!-- AC:END -->
