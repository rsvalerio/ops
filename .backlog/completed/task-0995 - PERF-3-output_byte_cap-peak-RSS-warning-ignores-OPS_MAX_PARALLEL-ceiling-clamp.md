---
id: TASK-0995
title: >-
  PERF-3: output_byte_cap peak-RSS warning ignores OPS_MAX_PARALLEL ceiling
  clamp
status: Done
assignee: []
created_date: '2026-05-04 22:00'
updated_date: '2026-05-05 00:15'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/results.rs:182-197`

**What**: `output_byte_cap()` reads `OPS_MAX_PARALLEL` directly with the
inline parser:

\`\`\`rust
let max_parallel: usize = std::env::var("OPS_MAX_PARALLEL")
    .ok()
    .and_then(|s| s.parse::<usize>().ok())
    .filter(|&n| n > 0)
    .unwrap_or(32);
\`\`\`

The parallel orchestrator (`parallel.rs::resolve_max_parallel`) does its own
parse with **two** additional behaviours: clamp to
`MAX_PARALLEL_CEILING = 1024`, and emit a `tracing::warn!` on out-of-range
values. So if an operator sets `OPS_MAX_PARALLEL=5000`:

- `parallel.rs` clamps to 1024 (the actual concurrency).
- `output_byte_cap` computes the peak warning against `5000 × cap × 2`,
  reporting a fictitious worst-case roughly 5× higher than what can
  actually occur.
- Conversely, an operator who sets a junk value sees the parallel layer
  fall back to 32 with a warn, while output_byte_cap silently accepts the
  junk (`unwrap_or(32)` swallows the parse error with no log).

**Why it matters**: The peak-RSS warning exists so operators have a
trustworthy "is my budget over 1 GiB?" signal. Computing it against an
unclamped value undermines that contract, and the silent fallback in
results.rs hides the same misconfiguration that parallel.rs already warns
loudly about — operators who fix the parallel warning still see no change
in the peak warning number.

Refactor so both modules share one `resolve_env_usize`-style helper (the
parallel.rs one already exists) and `output_byte_cap` consumes the
*resolved* (clamped, validated) max-parallel value.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 OPS_MAX_PARALLEL is parsed and clamped in exactly one place; output_byte_cap reuses that helper
- [ ] #2 test pins that an out-of-range OPS_MAX_PARALLEL produces a peak warning computed against the clamped (not the raw) value
<!-- AC:END -->
