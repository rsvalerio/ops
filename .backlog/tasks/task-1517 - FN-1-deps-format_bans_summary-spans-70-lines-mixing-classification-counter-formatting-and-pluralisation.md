---
id: TASK-1517
title: >-
  FN-1: deps format_bans_summary spans 70 lines mixing classification, counter
  formatting, and pluralisation
status: To Do
assignee:
  - TASK-1574
created_date: '2026-05-19 07:27'
updated_date: '2026-05-19 16:45'
labels:
  - code-review-rust
  - fn
  - dup
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/format.rs:282-351` (`format_bans_summary`)

**What**: `format_bans_summary` is 70 lines. It interleaves four responsibilities:

1. The empty-section short-circuit (283-287).
2. The four-bucket classifier loop (296-307) — already shares the `SeverityClass` enum with `colorize_severity` but the bucket counts are tracked as four parallel `usize` locals.
3. Four nearly-identical bucket-to-formatted-fragment blocks (309-342) each open-coding the same `if n == 1 { "" } else { "s" }` pluralisation.
4. The final summary `writeln!` (344-350).

Compressing 2–3 into a `for class in [Error, Warning, Info, Unknown] { … }` loop driven by `enum`-keyed counters (e.g. `EnumMap<SeverityClass, usize>` or a 4-element array) collapses the body to a handful of lines and makes adding a future severity class one line of work instead of four.

**Why it matters**: FN-1 (function > 50 lines). The function is read-only-on-screen but every cargo-deny severity change requires editing four spread-out blocks; concentrating the per-class shape in a single loop is the lower-defect surface.

<!-- scan confidence: function length verified — 70 lines per `awk '/^fn format_bans_summary/,/^}/' | wc -l` -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 format_bans_summary reduced below 50 lines (rule FN-1) by consolidating the four bucket counters and their per-bucket formatting
- [ ] #2 Plural-s shaping ('error/errors', 'warning/warnings', 'unknown severity/severities') driven by a single helper rather than four open-coded copies
- [ ] #3 bans_summary_unknown_severity_renders_distinctly_from_info and format_report_bans_plural_errors_and_warnings continue to pass without diff in expected output
<!-- AC:END -->
