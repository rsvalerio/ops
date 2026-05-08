---
id: TASK-1074
title: >-
  PATTERN-1: deps parse_upgrade_table header gate silently drops entries on
  column-name drift
status: Done
assignee: []
created_date: '2026-05-07 21:19'
updated_date: '2026-05-08 12:00'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse.rs:96`

**What**: Header detection gate uses `starts_with("name") && contains("old req")`. If cargo-edit renames either token (e.g. "Name" / "Old req" / "current req" / "old version"), `columns` stays `None` and every data row is silently skipped, returning `Vec::new()`.

**Why it matters**: Distinct from TASK-1026 (which fixed case-sensitivity in body header columns) — this is the gate that decides whether to record the separator row at all. Silent supply-chain "no upgrades" on cargo-edit format change is a fail-open visibility regression.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 On Some(0) with non-empty stdout that contains a ==== separator but no recognised header, emit tracing::warn! and bail (do not return empty silently)
- [x] #2 Match header tokens case-insensitively or by separator-row presence
- [x] #3 Regression test with a renamed header column asserts warn fires and entries are not silently empty
<!-- AC:END -->
