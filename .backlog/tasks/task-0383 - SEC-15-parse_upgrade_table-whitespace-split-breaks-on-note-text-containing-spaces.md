---
id: TASK-0383
title: >-
  SEC-15: parse_upgrade_table whitespace split breaks on note text containing
  spaces
status: Done
assignee:
  - TASK-0415
created_date: '2026-04-26 09:39'
updated_date: '2026-04-26 11:13'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse.rs:70`

**What**: The parser uses split_whitespace().collect() and then assumes positions 0-4 are columns and 5.. is the note. cargo upgrade --dry-run may emit notes that are not single tokens (e.g., upcoming "pinned by parent" hints), which would shift columns and corrupt parsing silently.

**Why it matters**: Upstream cargo upgrade output format is not stable/documented; any column addition will silently misclassify entries (UpgradeEntry fields swap places).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Calibrate column offsets from the header row (use byte-position of name, old req, etc.) and slice each row by those offsets
- [ ] #2 Tests covering rows with multi-word notes and rows with extra columns
<!-- AC:END -->
