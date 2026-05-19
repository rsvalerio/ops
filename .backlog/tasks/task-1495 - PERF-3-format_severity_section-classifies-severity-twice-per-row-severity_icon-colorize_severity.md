---
id: TASK-1495
title: >-
  PERF-3: format_severity_section classifies severity twice per row
  (severity_icon + colorize_severity)
status: To Do
assignee:
  - TASK-1574
created_date: '2026-05-18 17:29'
updated_date: '2026-05-19 16:45'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/format.rs:71-86`, `:250-273`

**What**: In the row loop of `format_severity_section`, every entry runs:

```rust
let icon = severity_icon(row.severity);                    // -> classify(severity)
... colorize_severity(icon, row.severity) ...              // -> classify(severity) again
```

Both helpers internally call `SeverityClass::classify(severity)`. `colorize_severity` additionally emits a `tracing::warn!` from inside the row loop when the class is `Unknown` — so a section with N unknown-severity entries fires N warnings *per render*, despite the doc comment on the helper claiming "a single warn per render".

The fix is straightforward: classify once at the top of the loop body, then drive both icon and style from the resulting `SeverityClass`. This collapses the double classify to one and turns the unknown-severity log into a per-row decision the caller controls (or, better, a per-section dedup so the operator log carries one warn per render as documented).

**Why it matters**:

- PERF-3 / OWN-8: extra match arm per row is cheap individually but the function is on the hot path of the report renderer and the duplication contradicts the explicit "one allocation per render" intent stated elsewhere in the file (PERF-3 / TASK-0802, TASK-0880 comments).
- Correctness: the existing `tracing::warn!` doc comment on `colorize_severity` says "a single warn per render so the operator log carries the offending value" — but the call site fires it inside the per-row loop, so a deny.toml with 20 unknown-severity entries spams 20 identical warns. The comment and the code disagree.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 format_severity_section classifies each row's severity exactly once and reuses the SeverityClass for icon + style
- [ ] #2 Unknown-severity tracing::warn fires at most once per render (matches the colorize_severity doc comment), not once per row
- [ ] #3 Existing format_report tests still pass; behaviour on known severities is unchanged byte-for-byte
<!-- AC:END -->
