---
id: TASK-1041
title: >-
  PATTERN-1: deps format_bans_summary buckets unknown / <missing-severity> bans
  into 'info' counter, masking fail-closed gate signal
status: Done
assignee: []
created_date: '2026-05-07 20:52'
updated_date: '2026-05-08 06:36'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/format.rs:281-317` (`format_bans_summary`)

**What**: The duplicate-crate summary line counts entries via two literal-string filters:

```rust
let errors = bans.iter().filter(|b| b.severity == "error").count();
let warnings = bans.iter().filter(|b| b.severity == "warning").count();
let others = bans.len() - errors - warnings;
```

`others` is then rendered as `"{others} info"` in dim style. That bucket silently absorbs:

* `<missing-severity>` (the `MISSING_SEVERITY_SENTINEL` from `parse.rs:560` — emitted when cargo-deny drops the `severity` field; `has_issues` treats this as actionable / fail-closed).
* Any future cargo-deny severity (`critical`, `notice`, …) that is unknown to the renderer — `has_issues` (lib.rs:230) also routes unknowns to fail-closed.

The supply-chain gate already uses fail-closed semantics for unknown / missing severities (TASK-0601 / TASK-0845), but the operator-facing summary line labels them as benign `info`, contradicting the gate decision and hiding the schema-drift signal that the parse-time `tracing::warn!` was meant to surface.

Sister code in `format.rs::SeverityClass::classify` already distinguishes `Unknown` and renders with a red `?`; the bans summary is the only renderer that doesn't.

**Why it matters**: PATTERN-1 — the summary line silently misclassifies fail-closed-bound bans as informational. An operator scanning the report sees "0 errors, 0 warnings, 1 info" and concludes "transitive, usually harmless"; meanwhile the gate fails the build. The two views must agree, or the surprise lands at CI exit time with no breadcrumb pointing at the actual diagnostic.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 format_bans_summary classifies entries via SeverityClass (or equivalent) so unknown / <missing-severity> render in a distinct bucket, not 'info'
- [x] #2 Regression test asserts a bans entry with severity '<missing-severity>' or an unknown like 'critical' renders distinctly from 'note'/'help'/'info'
<!-- AC:END -->
