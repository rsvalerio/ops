---
id: TASK-0975
title: >-
  ERR-7: cargo-update parse_update_output logs untrusted line via Display field,
  enabling log injection from crate names
status: Done
assignee:
  - TASK-1010
created_date: '2026-05-04 21:58'
updated_date: '2026-05-06 06:53'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:122-125, 242`

**What**: `tracing::warn!(line = %clean, "skipping cargo-update line ...")` (line 122) and `tracing::warn!(line, ...)` (line 242) format the cargo-update stderr line through the Display formatter. `strip_ansi` removes CSI escapes but leaves raw control characters (newlines, carriage returns, BEL, raw `\x1b` not followed by `[`). Crate names appear in these strings — adversary-published crates can therefore inject newlines into operator-facing logs. Sister sweep to TASK-0941 / TASK-0947 / TASK-0965 which fixed the same anti-pattern in cargo-toml workspace walk and core config loader.

**Why it matters**: The `cargo update --dry-run` data provider runs as part of `ops about` / metadata pipelines; a malicious crate name in a transitive dep can forge log records (fake severity lines, hide subsequent diagnostics) when the parser encounters drift. Both warns are reachable in normal operation: line 122 fires whenever cargo-update format drifts, line 242 fires whenever `Updating … -> …` carries trailing annotation tokens (the same drift TASK-0949 traces).

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Both tracing::warn! sites format line via Debug (?) instead of Display (%), so embedded newlines/control chars are escaped
- [ ] #2 Add a regression test that drives parse_update_output with a crate name containing \n / \x1b and asserts the captured log line does not contain a literal newline
<!-- AC:END -->
