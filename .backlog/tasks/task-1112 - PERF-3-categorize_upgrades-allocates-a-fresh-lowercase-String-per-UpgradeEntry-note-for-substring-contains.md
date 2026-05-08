---
id: TASK-1112
title: >-
  PERF-3: categorize_upgrades allocates a fresh lowercase String per
  UpgradeEntry note for substring contains
status: Done
assignee: []
created_date: '2026-05-07 21:49'
updated_date: '2026-05-08 06:17'
labels:
  - code-review-rust
  - PERF
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse.rs:274-277` (`categorize_upgrades`)

**What**: The is_incompatible check is `entry.note.as_deref().is_some_and(|n| n.to_ascii_lowercase().contains("incompatible"))`. For every parsed upgrade entry this allocates a brand-new `String` whose only purpose is a single `.contains()` check, then drops it. The hot path is `cargo upgrade --dry-run` output processing, which on an active workspace easily emits dozens of rows.

**Why it matters**: Same pattern called out in the rules-core PERF-3 examples (allocating per-iteration strings whose lifetime ends inside the iteration). Replace with a case-insensitive substring scan that walks the bytes/chars directly, e.g. a small helper `contains_ascii_ci(haystack, "incompatible")` that does a windowed `eq_ignore_ascii_case` against the literal needle. Comment at line 271 already calls out future wording drift like `"incompatible (semver)"`, so the scan stays needle-anchored.

**Why it's not duplicate**: TASK-1053 (workspace-wide `key.to_lowercase()` per env entry) is a different call site; TASK-1065 (`insert_thousands_separators`) is unrelated formatter; TASK-1035 (`left_pad_str`) is per-render padding. No existing task targets categorize_upgrades.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 categorize_upgrades does no per-row String allocation for the incompatibility check
- [ ] #2 Behaviour parity preserved — case-insensitive 'incompatible' substring still matches and 'incompatible (semver)' future-drift still classifies
<!-- AC:END -->
