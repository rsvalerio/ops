---
id: TASK-0989
title: >-
  DUP-3: deps has_issues defines is_actionable and is_actionable_ban that differ
  only in 'warning' handling, duplicating the severity table
status: Done
assignee: []
created_date: '2026-05-04 21:59'
updated_date: '2026-05-04 23:15'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/lib.rs:230-265`

**What**: `has_issues` declares two near-identical closures — `is_actionable` and `is_actionable_ban` — that differ only in whether `"warning"` is treated as actionable. Both repeat the exact same match-arm list for `note | help | info` benign and the same fail-closed-with-warn fallback for unknown severities. A future cargo-deny severity addition (e.g. a hypothetical `critical`, `notice`, or a renamed `info` variant) requires editing two separate match expressions in the same function, in lockstep — exactly the DUP-3 anti-pattern that TASK-0972 just flagged for `severity_icon` / `colorize_severity` in the sibling `format.rs`.

**Why it matters**: Drift between the two predicates inverts a supply-chain safety property. The bans gate is intentionally relaxed (warning = informational), but the relaxation logic should live as one parameter on a single helper, not as a copy-pasted second function. Co-locating the table also makes "what does cargo-deny severity X mean to ops?" answerable from one source of truth.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Single severity-classification helper takes a 'relax_warning' bool (or strongly-typed enum) and is invoked twice instead of two near-identical closures
- [ ] #2 Existing has_issues_* tests still pass; new test pins both relaxed and strict warning behaviour against the same helper
<!-- AC:END -->
