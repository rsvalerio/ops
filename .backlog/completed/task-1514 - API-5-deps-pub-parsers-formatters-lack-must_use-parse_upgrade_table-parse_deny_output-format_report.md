---
id: TASK-1514
title: >-
  API-5: deps pub parsers/formatters lack #[must_use] (parse_upgrade_table,
  parse_deny_output, format_report)
status: Done
assignee:
  - TASK-1648
created_date: '2026-05-19 06:39'
updated_date: '2026-05-25 19:03'
labels:
  - code-review-rust
  - api
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse/upgrade.rs:119`, `extensions-rust/deps/src/parse/deny.rs:258`, `extensions-rust/deps/src/format.rs:88`

**What**: Three pub fns whose entire purpose is their return value lack `#[must_use]`:
- `parse_upgrade_table(stdout: &str) -> Vec<UpgradeEntry>` — parses cargo-upgrade table
- `parse_deny_output(stderr: &str) -> DenyResult` — parses cargo-deny NDJSON
- `format_report(report: &DepsReport) -> String` — formats the dependency health report

**Why it matters**: These functions are pure transformations — the return value is the whole point. Without `#[must_use]`, callers can accidentally discard the result (e.g. `parse_upgrade_table(out);` with no binding) and the compiler will not warn. Adding the attribute is a zero-cost API-5 fix that catches silent drops at compile time.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 parse_upgrade_table, parse_deny_output, and format_report carry #[must_use] (with a short rationale string where idiomatic)
- [ ] #2 cargo clippy --all -- -D warnings stays clean
<!-- AC:END -->
