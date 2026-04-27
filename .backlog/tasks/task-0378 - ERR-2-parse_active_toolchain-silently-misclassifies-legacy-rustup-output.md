---
id: TASK-0378
title: 'ERR-2: parse_active_toolchain silently misclassifies legacy rustup output'
status: Done
assignee:
  - TASK-0421
created_date: '2026-04-26 09:38'
updated_date: '2026-04-27 16:10'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe.rs:22`

**What**: Parses only the first whitespace-separated token of the first line. Older rustup versions print stable-aarch64-apple-darwin (default) (works), but rustup >=1.28 may print a multi-line block including an "(active)" marker — the regex-free heuristic produces wrong toolchain strings.

**Why it matters**: Wrong toolchain → component installs land on a different toolchain than the user expects.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Use rustup show active-toolchain --quiet (single line, machine-readable) and trim
- [x] #2 Unit tests cover both the legacy and the current rustup output formats
<!-- AC:END -->
