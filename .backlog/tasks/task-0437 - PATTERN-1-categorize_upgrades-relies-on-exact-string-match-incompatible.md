---
id: TASK-0437
title: 'PATTERN-1: categorize_upgrades relies on exact-string match "incompatible"'
status: Done
assignee:
  - TASK-0533
created_date: '2026-04-28 04:43'
updated_date: '2026-04-28 17:50'
labels:
  - code-review-rust
  - patterns
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse.rs:173-189`

**What**: Categorisation depends on `entry.note.as_deref() == Some("incompatible")`. Any other note text — even a benign upstream change like "incompatible (semver bump)", "breaking", or a localized variant — flips the entry into the compatible bucket. The note column is itself best-effort (column-offset parsing per SEC-15/TASK-0383), so any drift in cargo-edit wording silently misclassifies breaking upgrades as compatible.

**Why it matters**: The compatible/incompatible split feeds the user-facing report and the has_issues gate in run_deps. Misclassification causes ops to recommend `cargo upgrade` for breaking upgrades, which is an actionable wrong answer — not just a display bug.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Switch to a substring/case-insensitive match (e.g. note.to_ascii_lowercase().contains("incompatible")) or a small allowlist and add a comment naming the cargo-edit source line being matched
- [x] #2 Add a regression test exercising at least the bare "incompatible" form plus one mixed-case / suffixed variant
<!-- AC:END -->
