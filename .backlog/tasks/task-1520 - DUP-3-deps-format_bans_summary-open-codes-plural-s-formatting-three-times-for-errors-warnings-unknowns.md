---
id: TASK-1520
title: >-
  DUP-3: deps format_bans_summary open-codes plural-s formatting three times for
  errors/warnings/unknowns
status: To Do
assignee:
  - TASK-1574
created_date: '2026-05-19 07:28'
updated_date: '2026-05-19 16:45'
labels:
  - code-review-rust
  - dup
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/format.rs:310-342` (`format_bans_summary`)

**What**: The function emits three nearly-identical fragments for the error / warning / unknown-severities buckets. Each one re-implements the same conditional pluralisation:

```rust
red(&format!("{} error{}", errors, if errors == 1 { "" } else { "s" }))
yellow(&format!("{} warning{}", warnings, if warnings == 1 { "" } else { "s" }))
red(&format!("{} unknown severit{}", unknowns, if unknowns == 1 { "y" } else { "ies" }))
```

The shape is the same: pick a colour, pick a singular/plural suffix, push if the bucket is non-zero. A `fn plural(n: usize, singular: &str, plural: &str)` helper (or a `Cow`-returning variant) plus a single iteration over `[(SeverityClass::Error, errors, "error", "errors"), …]` would collapse this to four data rows. The current shape means a typo in one bucket's pluralisation (e.g. "warning" → "warnings" off-by-one) goes uncaught by the others.

**Why it matters**: DUP-3 (5+ line repeated formatting fragments differing only in data). Related: TASK-0972 unified `SeverityClass` icon/style; this is the last hold-out where the per-class shape is still triplicated.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 format_bans_summary's three plural-shape format! blocks consolidated into a single iteration over a per-class table
- [ ] #2 Pluralisation conditional (n == 1) lives in one helper, not three open-coded copies
- [ ] #3 bans_summary_unknown_severity_renders_distinctly_from_info and format_report_bans_plural_errors_and_warnings continue to pass without expected-output diff
<!-- AC:END -->
